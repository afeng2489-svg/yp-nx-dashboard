import { useState, useEffect, useRef, useCallback } from 'react';
import {
  Palette,
  Image,
  Layout,
  Zap,
  Code2,
  FileCode,
  GitCompare,
  Play,
  Loader2,
  ChevronRight,
  CheckCircle,
  ExternalLink,
  X,
  Globe,
  FolderOpen,
  Copy,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import type { RawLine } from '@/stores/executionStore';
import { showError, showSuccess } from '@/lib/toast';
import { cn } from '@/lib/utils';

// ── 类型 ──────────────────────────────────────────────
type StepId = 'extract' | 'generate' | 'codify' | 'sync';
type ExtractSubStep = 'style' | 'layout' | 'animation';
type InputMode = 'file' | 'url';

interface Step {
  id: StepId;
  label: string;
  icon: React.ReactNode;
  description: string;
  gradient: string;
}

const STEPS: Step[] = [
  {
    id: 'extract',
    label: '提取规格',
    icon: <Image className="w-5 h-5" />,
    description: '从设计稿、代码或网站 URL 提取设计规格',
    gradient: 'from-blue-500 to-cyan-500',
  },
  {
    id: 'generate',
    label: '生成组件',
    icon: <Code2 className="w-5 h-5" />,
    description: '基于设计规格生成 React + Tailwind 组件',
    gradient: 'from-purple-500 to-pink-500',
  },
  {
    id: 'codify',
    label: '固化到项目',
    icon: <FileCode className="w-5 h-5" />,
    description: '将设计 Token 写入 tokens.css 和 tailwind.config.js',
    gradient: 'from-orange-500 to-amber-500',
  },
  {
    id: 'sync',
    label: '还原度检查',
    icon: <GitCompare className="w-5 h-5" />,
    description: '对比参考设计稿与代码实现，输出差异报告',
    gradient: 'from-emerald-500 to-green-500',
  },
];

// 各子工作流的文件模式字段定义
const EXTRACT_FILE_TABS = [
  {
    id: 'style' as ExtractSubStep,
    label: 'Style Extract',
    icon: <Palette className="w-4 h-4" />,
    wfName: 'style-extract',
    fields: [
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径（PNG/JPG/SVG）' },
      { key: 'code_path', label: '代码路径', desc: 'CSS/TSX/HTML 文件路径' },
    ],
  },
  {
    id: 'layout' as ExtractSubStep,
    label: 'Layout Extract',
    icon: <Layout className="w-4 h-4" />,
    wfName: 'layout-extract',
    fields: [
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径' },
      { key: 'html_path', label: 'HTML 路径', desc: 'HTML/TSX 文件路径' },
    ],
  },
  {
    id: 'animation' as ExtractSubStep,
    label: 'Animation Extract',
    icon: <Zap className="w-4 h-4" />,
    wfName: 'animation-extract',
    fields: [
      { key: 'css_path', label: 'CSS 路径', desc: 'CSS/SCSS/TSX 文件路径' },
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径（推断动画意图）' },
    ],
  },
];

// ── 工作流执行 Hook ──────────────────────────────────
function useWorkflowExecutor() {
  const { workflows, fetchWorkflows } = useWorkflowStore();
  const { startExecution } = useExecutionStore();
  const [running, setRunning] = useState<string | null>(null);

  useEffect(() => {
    fetchWorkflows();
  }, [fetchWorkflows]);

  // Returns executionId on success, null on failure
  const execute = async (
    wfName: string,
    variables: Record<string, string>,
  ): Promise<string | null> => {
    const wf = workflows.find((w) => w.name === wfName);
    if (!wf) {
      showError(`工作流 "${wfName}" 未找到，请重启后端以导入`);
      return null;
    }
    setRunning(wfName);
    try {
      const execution = await startExecution(wf.id, variables as Record<string, unknown>);
      showSuccess(`"${wfName}" 已启动`);
      return execution.id;
    } catch (e) {
      showError(`启动失败: ${e}`);
      return null;
    } finally {
      setRunning(null);
    }
  };

  return { running, execute };
}

// ── 实时执行输出面板（读 store，无独立 WebSocket）────────
function InlineExecPanel({
  executionId,
  onExtract,
  onClose,
}: {
  executionId: string;
  onExtract?: (key: string, value: string) => void;
  onClose?: () => void;
}) {
  const lines = useExecutionStore((s) => s.outputLines.get(executionId) ?? []);
  const execution = useExecutionStore((s) => s.executions.find((e) => e.id === executionId));
  const containerRef = useRef<HTMLDivElement>(null);
  const prevLenRef = useRef(0);
  const onExtractRef = useRef(onExtract);
  const navigate = useNavigate();

  useEffect(() => {
    onExtractRef.current = onExtract;
  }, [onExtract]);

  // 扫描新增行中的 EXTRACT: 模式
  useEffect(() => {
    const newLines = lines.slice(prevLenRef.current);
    prevLenRef.current = lines.length;
    for (const line of newLines) {
      if (line.type === 'output') {
        const m = line.content.match(/EXTRACT:(\w+)=(.+)/s);
        if (m) onExtractRef.current?.(m[1], m[2].trim());
      }
    }
  }, [lines]);

  // 自动滚到底部
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines]);

  const status = execution?.status ?? 'running';

  const copyAll = () => {
    const text = lines
      .map((l) => {
        if (l.type === 'stage_started') return `\n▶ ${l.stageName}`;
        if (l.type === 'stage_completed') return `✓ ${l.stageName}`;
        return l.content;
      })
      .join('\n');
    navigator.clipboard.writeText(text);
    showSuccess('已复制到剪贴板');
  };

  return (
    <div className="bg-card rounded-2xl border border-border/50 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border/50 bg-muted/30">
        <div className="flex items-center gap-2 text-sm font-medium">
          {(status === 'pending' || status === 'running') && (
            <Loader2 className="w-3.5 h-3.5 animate-spin text-blue-500" />
          )}
          {status === 'completed' && <CheckCircle className="w-3.5 h-3.5 text-green-500" />}
          {(status === 'failed' || status === 'cancelled') && (
            <X className="w-3.5 h-3.5 text-red-500" />
          )}
          <span>实时输出</span>
          <code className="text-xs text-muted-foreground font-mono">
            #{executionId.slice(0, 8)}
          </code>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={copyAll}
            title="复制全部"
            className="p-1.5 hover:bg-accent rounded-lg transition-colors"
          >
            <Copy className="w-3.5 h-3.5 text-muted-foreground" />
          </button>
          <button
            onClick={() => navigate('/executions')}
            title="查看完整执行记录"
            className="p-1.5 hover:bg-accent rounded-lg transition-colors"
          >
            <ExternalLink className="w-3.5 h-3.5 text-muted-foreground" />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 hover:bg-accent rounded-lg transition-colors"
            >
              <X className="w-3.5 h-3.5 text-muted-foreground" />
            </button>
          )}
        </div>
      </div>

      {/* 输出内容 */}
      <div
        ref={containerRef}
        className="max-h-96 overflow-y-auto p-4 space-y-0.5 font-mono text-xs bg-black/[0.03] dark:bg-white/[0.03]"
      >
        {lines.length === 0 && <p className="text-muted-foreground text-center py-6">等待输出…</p>}
        {lines.map((line: RawLine) => (
          <div
            key={line.id}
            className={cn(
              'leading-relaxed whitespace-pre-wrap break-words',
              line.type === 'stage_started' && 'text-blue-500 font-semibold pt-3 pb-0.5',
              line.type === 'stage_completed' && 'text-green-600 font-semibold pb-1',
              line.type === 'completed' && 'text-green-500 font-bold pt-2',
              line.type === 'error' && 'text-red-500',
              line.type === 'output' && 'text-foreground/75',
              line.type === 'info' && 'text-muted-foreground italic',
            )}
          >
            {line.type === 'stage_started' && `▶ ${line.stageName}`}
            {line.type === 'stage_completed' && `✓ ${line.stageName}`}
            {line.type !== 'stage_started' && line.type !== 'stage_completed' && line.content}
          </div>
        ))}
      </div>
    </div>
  );
}

// ── 通用输入表单 ──────────────────────────────────────
interface FieldDef {
  key: string;
  label: string;
  desc: string;
  required?: boolean;
  multiline?: boolean;
}

function FieldForm({
  fields,
  values,
  onChange,
}: {
  fields: FieldDef[];
  values: Record<string, string>;
  onChange: (k: string, v: string) => void;
}) {
  return (
    <div className="space-y-4">
      {fields.map((f) => (
        <div key={f.key}>
          <label className="block text-sm font-medium mb-1">
            {f.label}
            {f.required && <span className="text-red-500 ml-1">*</span>}
          </label>
          <p className="text-xs text-muted-foreground mb-1.5">{f.desc}</p>
          {f.multiline ? (
            <textarea
              className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none min-h-[100px] font-mono text-xs"
              placeholder={f.desc}
              value={values[f.key] ?? ''}
              onChange={(e) => onChange(f.key, e.target.value)}
            />
          ) : (
            <input
              type="text"
              className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
              placeholder={f.desc}
              value={values[f.key] ?? ''}
              onChange={(e) => onChange(f.key, e.target.value)}
            />
          )}
        </div>
      ))}
    </div>
  );
}

// ── 模式切换按钮 ──────────────────────────────────────
function ModeSwitcher({ mode, onChange }: { mode: InputMode; onChange: (m: InputMode) => void }) {
  return (
    <div className="flex items-center gap-1 p-1 bg-muted/50 rounded-xl w-fit">
      <button
        onClick={() => onChange('file')}
        className={cn(
          'flex items-center gap-2 px-4 py-1.5 rounded-lg text-sm font-medium transition-all',
          mode === 'file'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        )}
      >
        <FolderOpen className="w-4 h-4" />
        文件模式
      </button>
      <button
        onClick={() => onChange('url')}
        className={cn(
          'flex items-center gap-2 px-4 py-1.5 rounded-lg text-sm font-medium transition-all',
          mode === 'url'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        )}
      >
        <Globe className="w-4 h-4" />
        URL 模式
      </button>
    </div>
  );
}

// ── Step 1: 提取规格 ──────────────────────────────────
//
// 设计原则：
//   - style-extract 为必填，layout/animation 为可选增强
//   - 三个工作流相互独立，可针对不同 URL/文件分别运行
//   - 所有已收集的 spec 自动合并到 style_spec 传给下一步

const SPEC_KEY_MAP: Record<ExtractSubStep, string> = {
  style: 'style_spec',
  layout: 'layout_spec',
  animation: 'animation_spec',
};

const URL_PLACEHOLDERS: Record<ExtractSubStep, string> = {
  style: 'https://tailwindui.com — 提取颜色、字体、间距系统',
  layout: 'https://vercel.com — 提取网格布局、组件层级、响应式规则',
  animation: 'https://framer.com — 提取过渡时长、缓动函数、关键帧动画',
};

function ExtractStep({
  styleSpec,
  onStyleSpecChange,
  onReadyChange,
}: {
  styleSpec: string;
  onStyleSpecChange: (v: string) => void;
  onReadyChange?: (ready: boolean) => void;
}) {
  // 每个子工作流独立的模式和输入
  const [modes, setModes] = useState<Record<ExtractSubStep, InputMode>>({
    style: 'file',
    layout: 'file',
    animation: 'file',
  });
  const [fileValues, setFileValues] = useState<Record<ExtractSubStep, Record<string, string>>>({
    style: {},
    layout: {},
    animation: {},
  });
  const [urlValues, setUrlValues] = useState<Record<ExtractSubStep, string>>({
    style: '',
    layout: '',
    animation: '',
  });
  // 每个子工作流独立的执行 ID 和收集状态
  const [execIds, setExecIds] = useState<Partial<Record<ExtractSubStep, string>>>({});
  const [collected, setCollected] = useState<Partial<Record<ExtractSubStep, string>>>({});
  const { running, execute } = useWorkflowExecutor();

  // style 收集到了才算 ready
  useEffect(() => {
    onReadyChange?.(!!collected.style);
  }, [collected.style, onReadyChange]);

  // 任意 spec 变化时合并并通知父组件
  useEffect(() => {
    if (!collected.style) return;
    try {
      const base = JSON.parse(collected.style);
      if (collected.layout) {
        try {
          base.layout = JSON.parse(collected.layout);
        } catch {
          base.layout_raw = collected.layout;
        }
      }
      if (collected.animation) {
        try {
          base.animation = JSON.parse(collected.animation);
        } catch {
          base.animation_raw = collected.animation;
        }
      }
      onStyleSpecChange(JSON.stringify(base, null, 2));
    } catch {
      onStyleSpecChange(collected.style);
    }
  }, [collected, onStyleSpecChange]);

  const handleRun = async (tab: (typeof EXTRACT_FILE_TABS)[number]) => {
    const mode = modes[tab.id];
    const variables = mode === 'url' ? { url: urlValues[tab.id] } : (fileValues[tab.id] ?? {});
    if (mode === 'url' && !urlValues[tab.id]) return;

    const execId = await execute(tab.wfName, variables);
    if (execId) setExecIds((prev) => ({ ...prev, [tab.id]: execId }));
  };

  const makeExtractHandler = useCallback(
    (subStep: ExtractSubStep) => (key: string, value: string) => {
      const expectedKey = SPEC_KEY_MAP[subStep];
      if (key === expectedKey) {
        setCollected((prev) => ({ ...prev, [subStep]: value }));
        const label =
          subStep === 'style' ? '样式规格' : subStep === 'layout' ? '布局规格' : '动效规格';
        showSuccess(`${label} 已收集 ✓`);
      }
    },
    [],
  );

  const collectedCount = Object.keys(collected).length;

  return (
    <div className="space-y-4">
      {/* 总体进度提示 */}
      <div className="flex items-center gap-3 text-sm">
        <div className="flex items-center gap-2">
          {collectedCount === 0 && (
            <span className="text-muted-foreground">请先运行 Style Extract（必填）</span>
          )}
          {collectedCount > 0 && (
            <span className="text-green-600 font-medium">已收集 {collectedCount}/3 项规格</span>
          )}
        </div>
        {collected.style && !collected.layout && !collected.animation && (
          <span className="text-xs text-muted-foreground">· Layout / Animation 为可选增强</span>
        )}
      </div>

      {/* 三个独立工作流卡片 */}
      {EXTRACT_FILE_TABS.map((tab) => {
        const isRequired = tab.id === 'style';
        const isCollected = !!collected[tab.id];
        const isRunning = running === tab.wfName;
        const mode = modes[tab.id];
        const execId = execIds[tab.id] ?? null;

        return (
          <div
            key={tab.id}
            className={cn(
              'rounded-2xl border overflow-hidden transition-colors',
              isCollected ? 'border-green-500/30 bg-green-500/[0.02]' : 'border-border/50 bg-card',
            )}
          >
            {/* 卡片标题栏 */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-border/30">
              <div className="flex items-center gap-2.5">
                <span
                  className={cn(
                    'flex items-center justify-center w-7 h-7 rounded-lg',
                    isCollected
                      ? 'bg-green-500/10 text-green-600'
                      : 'bg-muted text-muted-foreground',
                  )}
                >
                  {isCollected ? <CheckCircle className="w-4 h-4" /> : tab.icon}
                </span>
                <span className="font-medium text-sm">{tab.label}</span>
                <span
                  className={cn(
                    'text-xs px-2 py-0.5 rounded-full font-medium',
                    isRequired ? 'bg-red-500/10 text-red-600' : 'bg-muted text-muted-foreground',
                  )}
                >
                  {isRequired ? '必填' : '可选'}
                </span>
                {isCollected && <span className="text-xs text-green-600 font-medium">已收集</span>}
              </div>
              {/* 模式切换 */}
              <ModeSwitcher
                mode={mode}
                onChange={(m) => setModes((prev) => ({ ...prev, [tab.id]: m }))}
              />
            </div>

            {/* 卡片内容 */}
            <div className="p-5 space-y-4">
              {mode === 'url' ? (
                <div>
                  <input
                    type="url"
                    className="w-full bg-background border border-border/50 rounded-xl px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                    placeholder={URL_PLACEHOLDERS[tab.id]}
                    value={urlValues[tab.id]}
                    onChange={(e) =>
                      setUrlValues((prev) => ({ ...prev, [tab.id]: e.target.value }))
                    }
                  />
                  {urlValues[tab.id] && (
                    <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground bg-blue-500/5 rounded-lg px-3 py-1.5">
                      <Globe className="w-3 h-3 text-blue-500 shrink-0" />
                      <code className="text-blue-600 font-mono truncate">{urlValues[tab.id]}</code>
                    </div>
                  )}
                </div>
              ) : (
                <FieldForm
                  fields={tab.fields}
                  values={fileValues[tab.id] ?? {}}
                  onChange={(k, v) =>
                    setFileValues((prev) => ({ ...prev, [tab.id]: { ...prev[tab.id], [k]: v } }))
                  }
                />
              )}

              <button
                onClick={() => handleRun(tab)}
                disabled={isRunning || (mode === 'url' && !urlValues[tab.id])}
                className="btn-primary flex items-center gap-2 disabled:opacity-50"
              >
                {isRunning ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Play className="w-4 h-4" />
                )}
                {isRunning ? '启动中…' : isCollected ? '重新运行' : `运行 ${tab.wfName}`}
              </button>
            </div>

            {/* 实时输出面板 */}
            {execId && (
              <div className="border-t border-border/30">
                <InlineExecPanel
                  executionId={execId}
                  onExtract={makeExtractHandler(tab.id)}
                  onClose={() => setExecIds((prev) => ({ ...prev, [tab.id]: undefined }))}
                />
              </div>
            )}
          </div>
        );
      })}

      {/* 合并规格预览（style 收集后才显示） */}
      {collected.style && (
        <div className="bg-card rounded-2xl border border-border/50 p-5">
          <div className="flex items-center justify-between mb-2">
            <label className="text-sm font-medium flex items-center gap-2">
              合并规格预览
              <span className="text-xs text-muted-foreground font-normal">
                （自动合并已收集的规格，传给下一步）
              </span>
            </label>
            <div className="flex gap-1.5 text-xs text-muted-foreground">
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.style ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                style ✓
              </span>
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.layout ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                layout {collected.layout ? '✓' : '–'}
              </span>
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.animation ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                animation {collected.animation ? '✓' : '–'}
              </span>
            </div>
          </div>
          <textarea
            className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none min-h-[80px]"
            placeholder='{"colors":{"primary":"#3B82F6",...},"typography":{...}}'
            value={styleSpec}
            onChange={(e) => onStyleSpecChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}

// ── Step 2: 生成组件 ──────────────────────────────────
function GenerateStep({ styleSpec }: { styleSpec: string }) {
  const [values, setValues] = useState<Record<string, string>>({
    style_spec: styleSpec,
    component_description: '',
    component_name: '',
    output_path: '',
  });
  const [activeExecutionId, setActiveExecutionId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();

  useEffect(() => {
    setValues((prev) => ({ ...prev, style_spec: styleSpec }));
  }, [styleSpec]);

  const fields: FieldDef[] = [
    {
      key: 'style_spec',
      label: 'style_spec',
      desc: '由提取规格阶段输出的设计 token JSON',
      required: true,
      multiline: true,
    },
    {
      key: 'component_description',
      label: '组件描述',
      desc: '描述要生成的组件，例如：带头像和操作按钮的用户卡片',
      required: true,
    },
    { key: 'component_name', label: '组件名（可选）', desc: 'PascalCase 命名，如 UserCard' },
    { key: 'output_path', label: '输出路径（可选）', desc: '默认为 src/components/<Name>.tsx' },
  ];

  const handleRun = async () => {
    const execId = await execute('generate', values);
    if (execId) setActiveExecutionId(execId);
  };

  return (
    <div className="space-y-5">
      <div className="bg-card rounded-2xl border border-border/50 p-5 space-y-5">
        <FieldForm
          fields={fields}
          values={values}
          onChange={(k, v) => setValues((prev) => ({ ...prev, [k]: v }))}
        />
        <button
          onClick={handleRun}
          disabled={running === 'generate'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'generate' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Code2 className="w-4 h-4" />
          )}
          {running === 'generate' ? '生成中…' : '生成组件'}
        </button>
      </div>
      {activeExecutionId && (
        <InlineExecPanel
          executionId={activeExecutionId}
          onClose={() => setActiveExecutionId(null)}
        />
      )}
    </div>
  );
}

// ── Step 3: 固化到项目 ──────────────────────────────────
function CodifyStep({ styleSpec }: { styleSpec: string }) {
  const [values, setValues] = useState<Record<string, string>>({
    style_spec: styleSpec,
    tokens_css_path: 'src/styles/tokens.css',
    tailwind_config_path: 'tailwind.config.js',
  });
  const [activeExecutionId, setActiveExecutionId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();

  useEffect(() => {
    setValues((prev) => ({ ...prev, style_spec: styleSpec }));
  }, [styleSpec]);

  const fields: FieldDef[] = [
    {
      key: 'style_spec',
      label: 'style_spec',
      desc: '设计 token JSON，将被写入 tokens.css 和 tailwind.config.js',
      required: true,
      multiline: true,
    },
    { key: 'tokens_css_path', label: 'tokens.css 路径', desc: 'CSS 变量文件输出路径' },
    {
      key: 'tailwind_config_path',
      label: 'tailwind.config.js 路径',
      desc: 'Tailwind 配置文件路径',
    },
  ];

  const handleRun = async () => {
    const execId = await execute('codify-style', values);
    if (execId) setActiveExecutionId(execId);
  };

  return (
    <div className="space-y-5">
      <div className="bg-card rounded-2xl border border-border/50 p-5 space-y-5">
        <div className="bg-amber-500/5 border border-amber-500/20 rounded-xl p-3 flex items-start gap-2 text-sm">
          <span className="text-amber-600 mt-0.5">⚠</span>
          <span className="text-amber-700">
            此操作将覆盖{' '}
            <code className="font-mono text-xs bg-amber-500/10 px-1 rounded">tokens.css</code> 和{' '}
            <code className="font-mono text-xs bg-amber-500/10 px-1 rounded">
              tailwind.config.js
            </code>{' '}
            中的设计相关配置。
          </span>
        </div>
        <FieldForm
          fields={fields}
          values={values}
          onChange={(k, v) => setValues((prev) => ({ ...prev, [k]: v }))}
        />
        <button
          onClick={handleRun}
          disabled={running === 'codify-style'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'codify-style' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <FileCode className="w-4 h-4" />
          )}
          {running === 'codify-style' ? '写入中…' : '固化到项目'}
        </button>
      </div>
      {activeExecutionId && (
        <InlineExecPanel
          executionId={activeExecutionId}
          onClose={() => setActiveExecutionId(null)}
        />
      )}
    </div>
  );
}

// ── Step 4: 还原度检查 ──────────────────────────────────
function SyncStep() {
  const [values, setValues] = useState<Record<string, string>>({
    reference_path: '',
    source_path: '',
    component_name: '',
  });
  const [activeExecutionId, setActiveExecutionId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();

  const fields: FieldDef[] = [
    {
      key: 'reference_path',
      label: '参考设计路径',
      desc: '设计稿图片路径或设计 token JSON 文件路径',
      required: true,
    },
    { key: 'source_path', label: '代码实现路径', desc: 'TSX/CSS 文件或目录路径', required: true },
    {
      key: 'component_name',
      label: '组件名（可选）',
      desc: '指定要检查的组件，留空则扫描整个目录',
    },
  ];

  const handleRun = async () => {
    const execId = await execute('design-sync', values);
    if (execId) setActiveExecutionId(execId);
  };

  return (
    <div className="space-y-5">
      <div className="bg-card rounded-2xl border border-border/50 p-5 space-y-5">
        <FieldForm
          fields={fields}
          values={values}
          onChange={(k, v) => setValues((prev) => ({ ...prev, [k]: v }))}
        />
        <button
          onClick={handleRun}
          disabled={running === 'design-sync'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'design-sync' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <GitCompare className="w-4 h-4" />
          )}
          {running === 'design-sync' ? '检查中…' : '开始检查'}
        </button>
      </div>
      {activeExecutionId && (
        <InlineExecPanel
          executionId={activeExecutionId}
          onClose={() => setActiveExecutionId(null)}
        />
      )}
    </div>
  );
}

// ── 主页面 ──────────────────────────────────────────────
export function UIDesignPage() {
  const navigate = useNavigate();
  const [activeStep, setActiveStep] = useState<StepId>('extract');
  const [styleSpec, setStyleSpec] = useState('');
  const [dismissGuide, setDismissGuide] = useState(false);
  const [extractReady, setExtractReady] = useState(false);

  const activeIdx = STEPS.findIndex((s) => s.id === activeStep);

  return (
    <div className="page-container space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-blue-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              UI 设计工作台
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">
            提取设计规格 → 生成组件 → 固化到项目 → 还原度检查
          </p>
        </div>
        <button
          onClick={() => navigate('/executions')}
          className="btn-secondary flex items-center gap-2"
        >
          <ExternalLink className="w-4 h-4" />
          查看执行记录
        </button>
      </div>

      {/* 使用指南 */}
      {!dismissGuide && (
        <div className="bg-gradient-to-r from-blue-500/5 via-purple-500/5 to-pink-500/5 border border-blue-500/20 rounded-2xl p-5 relative">
          <button
            onClick={() => setDismissGuide(true)}
            className="absolute top-3 right-3 p-1.5 hover:bg-blue-500/10 rounded-lg transition-colors"
          >
            <X className="w-4 h-4 text-muted-foreground" />
          </button>
          <p className="font-semibold text-sm mb-3 flex items-center gap-2">
            <Palette className="w-4 h-4 text-blue-500" />
            使用流程 · 支持两种输入模式
          </p>
          <div className="flex items-center gap-3 text-sm text-muted-foreground flex-wrap mb-3">
            {STEPS.map((s, i) => (
              <span key={s.id} className="flex items-center gap-2">
                <span className="px-2 py-0.5 rounded-full bg-white/50 dark:bg-black/20 text-xs font-medium">
                  {i + 1}. {s.label}
                </span>
                {i < STEPS.length - 1 && <ChevronRight className="w-3 h-3" />}
              </span>
            ))}
          </div>
          <div className="flex gap-4 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Globe className="w-3.5 h-3.5 text-blue-500" />
              <span>
                <strong className="text-foreground">URL 模式：</strong>输入网站地址，Claude
                自动抓取分析，结果实时显示在当前页面
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <FolderOpen className="w-3.5 h-3.5 text-purple-500" />
              <span>
                <strong className="text-foreground">文件模式：</strong>上传本地设计稿或代码路径
              </span>
            </div>
          </div>
        </div>
      )}

      <div className="flex gap-6">
        {/* 左侧 Step 导航 */}
        <div className="w-52 shrink-0 space-y-2">
          {STEPS.map((step, idx) => {
            const isActive = step.id === activeStep;
            const isDone = idx < activeIdx;
            return (
              <button
                key={step.id}
                onClick={() => setActiveStep(step.id)}
                className={cn(
                  'w-full flex items-center gap-3 px-4 py-3 rounded-xl text-left transition-all border',
                  isActive
                    ? 'border-primary/30 bg-gradient-to-r from-indigo-500/10 to-purple-500/10 shadow-sm'
                    : 'border-border/50 hover:bg-accent',
                )}
              >
                <div
                  className={cn(
                    'w-8 h-8 rounded-lg flex items-center justify-center shrink-0',
                    isActive
                      ? `bg-gradient-to-br ${step.gradient} text-white shadow-sm`
                      : isDone
                        ? 'bg-green-500/10 text-green-500'
                        : 'bg-muted text-muted-foreground',
                  )}
                >
                  {isDone ? <CheckCircle className="w-4 h-4" /> : step.icon}
                </div>
                <div className="min-w-0">
                  <p className={cn('text-sm font-medium truncate', isActive ? 'text-primary' : '')}>
                    {step.label}
                  </p>
                  <p className="text-xs text-muted-foreground truncate">
                    {step.description.slice(0, 18)}…
                  </p>
                </div>
              </button>
            );
          })}
        </div>

        {/* 右侧内容区 */}
        <div className="flex-1 min-w-0">
          {/* 当前步骤标题 */}
          <div
            className={cn(
              'flex items-center gap-3 p-4 rounded-2xl border mb-5',
              'bg-gradient-to-r from-card to-accent/20',
            )}
          >
            <div
              className={cn(
                'p-2.5 rounded-xl bg-gradient-to-br text-white shadow-lg',
                STEPS[activeIdx].gradient,
              )}
            >
              {STEPS[activeIdx].icon}
            </div>
            <div>
              <h2 className="font-semibold text-lg">
                Step {activeIdx + 1}: {STEPS[activeIdx].label}
              </h2>
              <p className="text-sm text-muted-foreground">{STEPS[activeIdx].description}</p>
            </div>
          </div>

          {activeStep === 'extract' && (
            <ExtractStep
              styleSpec={styleSpec}
              onStyleSpecChange={setStyleSpec}
              onReadyChange={setExtractReady}
            />
          )}
          {activeStep === 'generate' && <GenerateStep styleSpec={styleSpec} />}
          {activeStep === 'codify' && <CodifyStep styleSpec={styleSpec} />}
          {activeStep === 'sync' && <SyncStep />}

          {/* 下一步导航 */}
          {activeIdx < STEPS.length - 1 && (
            <div className="mt-5 flex justify-end">
              <button
                onClick={() => setActiveStep(STEPS[activeIdx + 1].id)}
                disabled={activeStep === 'extract' && !extractReady}
                className="btn-secondary flex items-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed"
                title={
                  activeStep === 'extract' && !extractReady ? '请先完成 Style Extract' : undefined
                }
              >
                下一步：{STEPS[activeIdx + 1].label}
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
