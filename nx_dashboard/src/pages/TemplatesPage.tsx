import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { X, Play, Loader2 } from 'lucide-react';
import { TemplateCard, TemplateCardSkeleton } from '@/components/workflow/TemplateCard';
import { useTemplateStore, TEMPLATE_CATEGORIES, type TemplateSummary, type TemplateCategory, type Template } from '@/stores/templateStore';
import { useExecutionStore } from '@/stores/executionStore';
import { cn } from '@/lib/utils';

// ── Launch Dialog ─────────────────────────────────────────
interface LaunchDialogProps {
  template: Template;
  onClose: () => void;
  onLaunch: (variables: Record<string, string>) => Promise<void>;
}

function LaunchDialog({ template, onClose, onLaunch }: LaunchDialogProps) {
  // Variables with empty string value are required inputs
  const requiredInputs = Object.entries(template.variables ?? {}).filter(
    ([, v]) => v === '' || v === null
  );

  const [values, setValues] = useState<Record<string, string>>(() =>
    Object.fromEntries(requiredInputs.map(([k]) => [k, '']))
  );
  const [launching, setLaunching] = useState(false);

  const canLaunch = requiredInputs.every(([k]) => values[k]?.trim());

  const handleLaunch = async () => {
    if (!canLaunch || launching) return;
    setLaunching(true);
    try {
      await onLaunch(values);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-2xl w-full max-w-md border border-border/50 overflow-hidden animate-scale-in">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <div>
            <h2 className="text-lg font-semibold">{template.name}</h2>
            <p className="text-xs text-muted-foreground mt-0.5">{template.description}</p>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-accent transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Inputs */}
        <div className="px-6 py-5 space-y-4">
          {requiredInputs.length === 0 ? (
            <p className="text-sm text-muted-foreground">该工作流无需额外输入，直接点击执行。</p>
          ) : (
            requiredInputs.map(([key]) => (
              <div key={key} className="space-y-1.5">
                <label className="text-sm font-medium capitalize">
                  {key === 'target' ? '审查目标' : key}
                  <span className="text-red-500 ml-1">*</span>
                </label>
                <textarea
                  value={values[key] ?? ''}
                  onChange={(e) => setValues((v) => ({ ...v, [key]: e.target.value }))}
                  placeholder={
                    key === 'target'
                      ? '文件路径（如 nx_api/src/routes/teams.rs）或功能描述'
                      : `请输入 ${key}`
                  }
                  className="w-full px-3 py-2.5 text-sm bg-muted/50 border border-border/50 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/30 resize-none"
                  rows={3}
                />
              </div>
            ))
          )}
        </div>

        {/* Actions */}
        <div className="px-6 py-4 border-t border-border/50 flex gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-xl text-muted-foreground hover:text-foreground hover:bg-accent transition-all"
          >
            取消
          </button>
          <button
            onClick={handleLaunch}
            disabled={!canLaunch || launching}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium rounded-xl transition-all',
              canLaunch && !launching
                ? 'bg-gradient-to-r from-indigo-500 to-purple-500 text-white shadow-lg shadow-indigo-500/25 hover:shadow-indigo-500/40'
                : 'bg-muted text-muted-foreground cursor-not-allowed'
            )}
          >
            {launching ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>启动中...</span>
              </>
            ) : (
              <>
                <Play className="w-4 h-4" />
                <span>执行</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────
export function TemplatesPage() {
  const navigate = useNavigate();
  const {
    templates,
    loading,
    error,
    selectedCategory,
    fetchTemplates,
    fetchTemplatesByCategory,
    getTemplate,
    instantiateTemplate,
  } = useTemplateStore();
  const { startExecution } = useExecutionStore();

  const [launchingTemplate, setLaunchingTemplate] = useState<Template | null>(null);

  useEffect(() => {
    fetchTemplates();
  }, []);

  const handleCategoryChange = (category: TemplateCategory | 'all') => {
    if (category === 'all') {
      fetchTemplates();
    } else {
      fetchTemplatesByCategory(category);
    }
  };

  const handleLaunch = useCallback(async (summary: TemplateSummary) => {
    const template = await getTemplate(summary.id);
    if (template) {
      setLaunchingTemplate(template);
    } else {
      toast.error('获取模板详情失败');
    }
  }, [getTemplate]);

  const handleExecute = useCallback(async (variables: Record<string, string>) => {
    if (!launchingTemplate) return;
    try {
      // 1. Instantiate template → create workflow
      const result = await instantiateTemplate(launchingTemplate.id);
      // 2. Start execution with user-provided variables
      const execution = await startExecution(result.workflow_id, variables);
      setLaunchingTemplate(null);
      toast.success('工作流已启动', { description: `正在执行: ${launchingTemplate.name}` });
      // 3. Navigate to executions page and auto-open the new execution
      navigate('/executions', { state: { openExecutionId: execution.id } });
    } catch {
      toast.error('启动失败，请重试');
    }
  }, [launchingTemplate, instantiateTemplate, startExecution, navigate]);

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">工作流模板</h1>
          <p className="text-sm text-muted-foreground mt-1">
            选择模板直接启动执行
          </p>
        </div>
      </div>

      {/* Category tabs */}
      <div className="px-4 border-b flex gap-2 overflow-x-auto">
        <button
          onClick={() => handleCategoryChange('all')}
          className={`px-3 py-2 text-sm rounded-lg transition-colors ${
            selectedCategory === null
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-accent'
          }`}
        >
          全部
        </button>
        {TEMPLATE_CATEGORIES.map((cat) => (
          <button
            key={cat.value}
            onClick={() => handleCategoryChange(cat.value)}
            className={`px-3 py-2 text-sm rounded-lg transition-colors ${
              selectedCategory === cat.value
                ? 'bg-primary text-primary-foreground'
                : 'hover:bg-accent'
            }`}
          >
            {cat.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4">
        {loading && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {[1, 2, 3].map((i) => (
              <TemplateCardSkeleton key={i} />
            ))}
          </div>
        )}
        {error && (
          <div className="flex items-center justify-center h-64 text-destructive">
            {error}
          </div>
        )}
        {!loading && !error && templates.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <p>暂无模板</p>
          </div>
        )}
        {!loading && !error && templates.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {templates.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                onLaunch={() => handleLaunch(template)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Launch Dialog */}
      {launchingTemplate && (
        <LaunchDialog
          template={launchingTemplate}
          onClose={() => setLaunchingTemplate(null)}
          onLaunch={handleExecute}
        />
      )}
    </div>
  );
}
