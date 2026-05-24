import { useState, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Zap,
  Image,
  Link,
  FileText,
  ChevronDown,
  ChevronUp,
  Loader2,
  Upload,
  X,
  Check,
  Sparkles,
  AlertCircle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { API_BASE_URL } from '@/api/constants';
import { useExecutionStore } from '@/stores/executionStore';
import { showError, showSuccess } from '@/lib/toast';

interface SuggestedComponent {
  name: string;
  component_type: string;
  description: string;
}

interface SuggestedDataModel {
  name: string;
  fields: string[];
  description: string;
}

interface ParseIntentResult {
  page_name?: string | null;
  description?: string | null;
  suggested_components?: SuggestedComponent[] | null;
  suggested_data_models?: SuggestedDataModel[] | null;
  confidence?: number | null;
  needs_clarification?: boolean | null;
  suggestions?: string[] | null;
}

async function readFileAsText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ''));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file);
  });
}

export function QuickLaunchPage() {
  const navigate = useNavigate();
  const { startExecution } = useExecutionStore();

  const [goal, setGoal] = useState('');
  const [context, setContext] = useState('');
  const [referenceUrl, setReferenceUrl] = useState('');
  const [screenshotFile, setScreenshotFile] = useState<File | null>(null);
  const [screenshotPreview, setScreenshotPreview] = useState<string | null>(null);
  const [apiDocFile, setApiDocFile] = useState<File | null>(null);
  const [apiDocContent, setApiDocContent] = useState<string | null>(null);

  const [showStructured, setShowStructured] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [enableRag, setEnableRag] = useState(true);
  const [reuseComponents, setReuseComponents] = useState(true);

  const [submitting, setSubmitting] = useState(false);
  const [parseResult, setParseResult] = useState<ParseIntentResult | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);
  const [showClarification, setShowClarification] = useState(false);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const apiDocInputRef = useRef<HTMLInputElement>(null);
  const dropZoneRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);

  const handleFileDrop = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(false);
    const file = e.dataTransfer.files[0];
    if (file && file.type.startsWith('image/')) {
      setScreenshotFile(file);
      const reader = new FileReader();
      reader.onload = () => setScreenshotPreview(reader.result as string);
      reader.readAsDataURL(file);
    }
  }, []);

  const handleScreenshotSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setScreenshotFile(file);
      const reader = new FileReader();
      reader.onload = () => setScreenshotPreview(reader.result as string);
      reader.readAsDataURL(file);
    }
  }, []);

  const handleApiDocSelect = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setApiDocFile(file);
      try {
        const text = await readFileAsText(file);
        setApiDocContent(text);
      } catch {
        setApiDocContent(null);
        showError('无法读取 API 文档文件');
      }
    }
  }, []);

  const removeScreenshot = useCallback(() => {
    setScreenshotFile(null);
    setScreenshotPreview(null);
    if (fileInputRef.current) fileInputRef.current.value = '';
  }, []);

  const removeApiDoc = useCallback(() => {
    setApiDocFile(null);
    setApiDocContent(null);
    if (apiDocInputRef.current) apiDocInputRef.current.value = '';
  }, []);

  const buildParsePayload = useCallback(async () => {
    const payload: Record<string, string> = {
      input: goal.trim(),
    };
    if (context.trim()) payload.context = context.trim();
    if (referenceUrl.trim()) payload.reference_url = referenceUrl.trim();
    if (screenshotPreview) payload.screenshot_url = screenshotPreview;
    if (apiDocContent) payload.api_doc = apiDocContent;
    return payload;
  }, [goal, context, referenceUrl, screenshotPreview, apiDocContent]);

  const handleParseIntent = useCallback(async () => {
    if (!goal.trim()) {
      showError('请描述你想要的页面');
      return;
    }
    setSubmitting(true);
    setShowClarification(false);
    setShowConfirm(false);
    try {
      const payload = await buildParsePayload();
      const res = await fetch(`${API_BASE_URL}/api/v1/ai/parse-intent`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      const result: ParseIntentResult = data.data ?? data;
      setParseResult(result);

      if (result.needs_clarification) {
        setShowClarification(true);
      } else {
        setShowConfirm(true);
      }
    } catch (e) {
      showError(`意图解析失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setSubmitting(false);
    }
  }, [goal, buildParsePayload]);

  const handleLaunch = useCallback(async () => {
    setShowConfirm(false);
    setSubmitting(true);
    try {
      const variables: Record<string, unknown> = {
        goal: goal.trim(),
        context: context.trim(),
        reference_url: referenceUrl.trim(),
        enable_rag: enableRag,
        reuse_components: reuseComponents,
      };
      if (screenshotPreview) variables.screenshot = screenshotPreview;
      if (apiDocContent) variables.api_doc = apiDocContent;
      if (parseResult?.page_name) variables.page_name = parseResult.page_name;
      if (parseResult?.suggested_components?.length) {
        variables.suggested_components = parseResult.suggested_components;
      }
      if (parseResult?.suggested_data_models?.length) {
        variables.suggested_data_models = parseResult.suggested_data_models;
      }

      await startExecution('page-generate', variables);
      showSuccess(
        '页面生成已启动',
        `页面名: ${parseResult?.page_name || goal.trim().slice(0, 30)}`,
      );
      navigate('/executions');
    } catch (e) {
      showError(`启动失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setSubmitting(false);
    }
  }, [
    goal,
    context,
    referenceUrl,
    screenshotPreview,
    apiDocContent,
    parseResult,
    enableRag,
    reuseComponents,
    startExecution,
    navigate,
  ]);

  const canSubmit = goal.trim().length > 0;
  const components = parseResult?.suggested_components ?? [];
  const dataModels = parseResult?.suggested_data_models ?? [];

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-3xl mx-auto px-6 py-12">
        <div className="text-center mb-10">
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-gradient-to-r from-red-500/10 to-rose-500/10 border border-red-500/20 mb-4">
            <Sparkles className="w-4 h-4 text-red-500" />
            <span className="text-sm font-medium text-red-600">AI 页面生成</span>
          </div>
          <h1 className="text-2xl font-bold mb-2">快速生成页面</h1>
          <p className="text-muted-foreground text-sm max-w-md mx-auto">
            用自然语言描述你想创建的页面，AI 将自动生成完整的前端代码。支持截图、参考 URL 和 API 文档作为设计参考。
          </p>
        </div>

        <div className="space-y-4">
          <div className="relative">
            <textarea
              className="w-full bg-card border border-border/50 rounded-2xl px-5 py-4 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/20 focus:border-red-500/30 resize-none min-h-[140px] placeholder:text-muted-foreground transition-all"
              placeholder="描述你想要的页面…&#10;&#10;例如: 创建一个数据仪表盘页面，包含销售额趋势图、用户分布地图和最近订单列表。需要从 /api/orders 获取数据。"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              autoFocus
            />
            <span className="absolute bottom-3 right-4 text-xs text-muted-foreground">
              {goal.length} 字
            </span>
          </div>

          <textarea
            className="w-full bg-card/50 border border-border/30 rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/10 focus:border-red-500/20 resize-none min-h-[60px] placeholder:text-muted-foreground transition-all"
            placeholder="补充上下文 (可选) — 项目类型、技术偏好、设计风格等"
            value={context}
            onChange={(e) => setContext(e.target.value)}
          />
        </div>

        <div className="mt-6">
          <button
            onClick={() => setShowStructured((v) => !v)}
            className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            {showStructured ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
            更多输入选项
          </button>

          {showStructured && (
            <div className="mt-4 space-y-4 p-5 bg-card/30 rounded-2xl border border-border/30">
              <div>
                <label className="flex items-center gap-2 text-sm font-medium mb-2">
                  <Image className="w-4 h-4 text-red-500" />
                  截图参考
                </label>
                {screenshotPreview ? (
                  <div className="relative inline-block">
                    <img
                      src={screenshotPreview}
                      alt="截图预览"
                      className="max-h-40 rounded-xl border border-border/50"
                    />
                    <button
                      onClick={removeScreenshot}
                      className="absolute -top-2 -right-2 p-1 rounded-full bg-card border border-border/50 shadow-sm hover:bg-accent transition-colors"
                    >
                      <X className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ) : (
                  <div
                    ref={dropZoneRef}
                    onDragOver={(e) => {
                      e.preventDefault();
                      setIsDragging(true);
                    }}
                    onDragLeave={() => setIsDragging(false)}
                    onDrop={handleFileDrop}
                    className={cn(
                      'flex flex-col items-center justify-center gap-2 p-6 rounded-xl border-2 border-dashed transition-colors cursor-pointer',
                      isDragging
                        ? 'border-red-500/50 bg-red-500/5'
                        : 'border-border/50 hover:border-red-500/30 hover:bg-red-500/5',
                    )}
                    onClick={() => fileInputRef.current?.click()}
                  >
                    <Upload className="w-6 h-6 text-muted-foreground" />
                    <p className="text-sm text-muted-foreground text-center">
                      {isDragging ? '释放以上传' : '拖拽截图到此处，或点击选择'}
                    </p>
                    <p className="text-xs text-muted-foreground/60">支持 PNG、JPG</p>
                  </div>
                )}
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={handleScreenshotSelect}
                />
              </div>

              <div>
                <label className="flex items-center gap-2 text-sm font-medium mb-2">
                  <Link className="w-4 h-4 text-red-500" />
                  参考 URL
                </label>
                <input
                  type="url"
                  className="w-full bg-background border border-border/50 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/10 focus:border-red-500/20 transition-all placeholder:text-muted-foreground"
                  placeholder="https://example.com/dashboard — 参考此页面的设计风格"
                  value={referenceUrl}
                  onChange={(e) => setReferenceUrl(e.target.value)}
                />
              </div>

              <div>
                <label className="flex items-center gap-2 text-sm font-medium mb-2">
                  <FileText className="w-4 h-4 text-red-500" />
                  API 文档上传
                </label>
                {apiDocFile ? (
                  <div className="flex items-center gap-3 p-3 bg-background rounded-xl border border-border/50">
                    <FileText className="w-5 h-5 text-red-500 flex-shrink-0" />
                    <span className="text-sm flex-1 truncate">{apiDocFile.name}</span>
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      {(apiDocFile.size / 1024).toFixed(1)} KB
                    </span>
                    <button
                      onClick={removeApiDoc}
                      className="p-1 rounded-lg hover:bg-accent transition-colors flex-shrink-0"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => apiDocInputRef.current?.click()}
                    className="flex items-center gap-2 w-full p-3 rounded-xl border border-border/50 hover:border-red-500/30 hover:bg-red-500/5 transition-colors text-sm text-muted-foreground"
                  >
                    <Upload className="w-4 h-4" />
                    点击上传 API 文档 (JSON/YAML)
                  </button>
                )}
                <input
                  ref={apiDocInputRef}
                  type="file"
                  accept=".json,.yaml,.yml,.txt,.md"
                  className="hidden"
                  onChange={handleApiDocSelect}
                />
              </div>

              <div>
                <button
                  onClick={() => setShowAdvanced((v) => !v)}
                  className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  {showAdvanced ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                  高级选项
                </button>

                {showAdvanced && (
                  <div className="mt-3 space-y-3 pl-2">
                    <div className="flex items-center justify-between">
                      <div>
                        <span className="text-sm">启用 RAG 知识库检索</span>
                        <p className="text-xs text-muted-foreground mt-0.5">
                          注入项目组件目录和技术栈文档
                        </p>
                      </div>
                      <label className="relative inline-flex items-center cursor-pointer">
                        <input
                          type="checkbox"
                          className="sr-only peer"
                          checked={enableRag}
                          onChange={(e) => setEnableRag(e.target.checked)}
                        />
                        <div className="w-9 h-5 bg-border/50 peer-checked:bg-red-500 rounded-full transition-colors after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-4" />
                      </label>
                    </div>
                    <div className="flex items-center justify-between">
                      <div>
                        <span className="text-sm">优先复用已有组件</span>
                        <p className="text-xs text-muted-foreground mt-0.5">
                          从组件目录中匹配可复用的组件
                        </p>
                      </div>
                      <label className="relative inline-flex items-center cursor-pointer">
                        <input
                          type="checkbox"
                          className="sr-only peer"
                          checked={reuseComponents}
                          onChange={(e) => setReuseComponents(e.target.checked)}
                        />
                        <div className="w-9 h-5 bg-border/50 peer-checked:bg-red-500 rounded-full transition-colors after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-4" />
                      </label>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="mt-6 flex justify-center">
          <button
            onClick={handleParseIntent}
            disabled={!canSubmit || submitting}
            className="inline-flex items-center gap-2 px-8 py-3 rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white font-medium shadow-lg shadow-red-500/25 hover:from-red-600 hover:to-rose-600 disabled:opacity-50 disabled:cursor-not-allowed transition-all text-sm"
          >
            {submitting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                解析中…
              </>
            ) : (
              <>
                <Zap className="w-4 h-4" />
                生成页面
              </>
            )}
          </button>
        </div>

        {showClarification && parseResult && (
          <div className="fixed inset-0 z-50 flex items-center justify-center">
            <div
              className="absolute inset-0 bg-black/40 backdrop-blur-sm"
              onClick={() => setShowClarification(false)}
            />
            <div className="relative w-full max-w-md mx-4 bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
              <div className="px-6 py-4 border-b border-border/50 bg-gradient-to-r from-amber-500/5 to-orange-500/5">
                <div className="flex items-center gap-3">
                  <AlertCircle className="w-5 h-5 text-amber-500" />
                  <div>
                    <h2 className="font-semibold">需要更多信息</h2>
                    <p className="text-xs text-muted-foreground">AI 无法确定你的需求，请补充描述</p>
                  </div>
                  <button
                    onClick={() => setShowClarification(false)}
                    className="ml-auto p-2 rounded-lg hover:bg-accent transition-colors"
                  >
                    <X className="w-5 h-5" />
                  </button>
                </div>
              </div>
              <div className="p-6 space-y-3">
                {(parseResult.suggestions ?? []).map((s) => (
                  <p key={s} className="text-sm text-muted-foreground">
                    • {s}
                  </p>
                ))}
                {(parseResult.suggestions ?? []).length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    请提供更具体的页面描述，例如包含哪些模块、数据来源和交互行为。
                  </p>
                )}
              </div>
              <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50">
                <button onClick={() => setShowClarification(false)} className="btn-primary">
                  继续编辑
                </button>
              </div>
            </div>
          </div>
        )}

        {showConfirm && parseResult && (
          <div className="fixed inset-0 z-50 flex items-center justify-center">
            <div
              className="absolute inset-0 bg-black/40 backdrop-blur-sm"
              onClick={() => setShowConfirm(false)}
            />
            <div className="relative w-full max-w-md mx-4 bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
              <div className="px-6 py-4 border-b border-border/50 bg-gradient-to-r from-red-500/5 to-rose-500/5">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-xl bg-gradient-to-br from-red-500 to-rose-500 shadow-lg shadow-red-500/25">
                    <Check className="w-4 h-4 text-white" />
                  </div>
                  <div>
                    <h2 className="font-semibold">确认页面生成</h2>
                    <p className="text-xs text-muted-foreground">AI 已解析你的需求</p>
                  </div>
                  <button
                    onClick={() => setShowConfirm(false)}
                    className="ml-auto p-2 rounded-lg hover:bg-accent transition-colors"
                  >
                    <X className="w-5 h-5" />
                  </button>
                </div>
              </div>
              <div className="p-6 space-y-4">
                <div>
                  <span className="text-xs text-muted-foreground">页面名称</span>
                  <p className="text-sm font-medium mt-0.5">
                    {parseResult.page_name || goal.trim().slice(0, 40)}
                  </p>
                </div>
                {parseResult.description && (
                  <div>
                    <span className="text-xs text-muted-foreground">描述</span>
                    <p className="text-sm mt-0.5 text-muted-foreground">{parseResult.description}</p>
                  </div>
                )}
                {components.length > 0 && (
                  <div>
                    <span className="text-xs text-muted-foreground">
                      组件列表 ({components.length})
                    </span>
                    <div className="flex flex-wrap gap-1.5 mt-1">
                      {components.map((c) => (
                        <span
                          key={c.name}
                          className="px-2.5 py-1 rounded-lg bg-red-500/10 text-red-600 text-xs font-medium"
                          title={c.description}
                        >
                          {c.name}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
                {dataModels.length > 0 && (
                  <div>
                    <span className="text-xs text-muted-foreground">
                      数据模型 ({dataModels.length})
                    </span>
                    <div className="flex flex-wrap gap-1.5 mt-1">
                      {dataModels.map((dm) => (
                        <span
                          key={dm.name}
                          className="px-2.5 py-1 rounded-lg bg-amber-500/10 text-amber-600 text-xs font-medium"
                          title={dm.description}
                        >
                          {dm.name}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
              <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50">
                <button onClick={() => setShowConfirm(false)} className="btn-secondary">
                  修改
                </button>
                <button
                  onClick={handleLaunch}
                  disabled={submitting}
                  className="btn-primary flex items-center gap-2 bg-gradient-to-r from-red-500 to-rose-500 border-0"
                >
                  {submitting ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Zap className="w-4 h-4" />
                  )}
                  {submitting ? '启动中…' : '确认生成'}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
