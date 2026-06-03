import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Play, Loader2, CheckSquare, Trash2 } from 'lucide-react';
import { TemplateCard, TemplateCardSkeleton } from '@/components/workflow/TemplateCard';
import {
  useTemplateStore,
  TEMPLATE_CATEGORIES,
  type TemplateSummary,
  type TemplateCategory,
  type Template,
} from '@/stores/templateStore';
import { useExecutionStore } from '@/stores/executionStore';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import { fieldLabel } from '@/components/workflow/launchFormUtils';
import { FormField, FormSection, formTextareaClass } from '@/components/ui/formStyles';
import { cn } from '@/lib/utils';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { Pagination } from '@/components/ui/Pagination';

const PAGE_SIZE = 6;

// ── Launch Dialog ─────────────────────────────────────────
interface LaunchDialogProps {
  template: Template;
  onClose: () => void;
  onLaunch: (variables: Record<string, string>) => Promise<void>;
}

function LaunchDialog({ template, onClose, onLaunch }: LaunchDialogProps) {
  // Variables with empty string value are required inputs
  const requiredInputs = Object.entries(template.variables ?? {}).filter(
    ([, v]) => v === '' || v === null,
  );

  const [values, setValues] = useState<Record<string, string>>(() =>
    Object.fromEntries(requiredInputs.map(([k]) => [k, ''])),
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
    <LaunchModalShell
      onClose={onClose}
      title={template.name}
      subtitle={template.description}
      icon={<Play />}
      accent="indigo"
      size="md"
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={handleLaunch}
          submitLabel="执行"
          submitting={launching}
          disabled={!canLaunch}
          submitIcon={!launching ? <Play className="h-4 w-4" /> : undefined}
          hint={requiredInputs.length > 0 ? '标有 * 的字段为必填' : undefined}
        />
      }
    >
      {requiredInputs.length === 0 ? (
        <p className="py-8 text-center text-sm text-muted-foreground">
          无需额外输入，点击下方按钮即可运行。
        </p>
      ) : (
        <FormSection title="运行参数">
          {requiredInputs.map(([key]) => (
            <FormField key={key} label={fieldLabel(key)} required>
              <textarea
                value={values[key] ?? ''}
                onChange={(e) => setValues((v) => ({ ...v, [key]: e.target.value }))}
                placeholder={
                  key === 'target'
                    ? '文件路径（如 nx_api/src/routes/teams.rs）或功能描述'
                    : `请输入${fieldLabel(key)}`
                }
                className={formTextareaClass}
                rows={3}
              />
            </FormField>
          ))}
        </FormSection>
      )}
    </LaunchModalShell>
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
    deleteTemplates,
  } = useTemplateStore();
  const { startExecution } = useExecutionStore();

  const [launchingTemplate, setLaunchingTemplate] = useState<Template | null>(null);
  const [page, setPage] = useState(1);
  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [confirmBatchDelete, setConfirmBatchDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    fetchTemplates();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleCategoryChange = (category: TemplateCategory | 'all') => {
    setPage(1);
    if (category === 'all') {
      fetchTemplates();
    } else {
      fetchTemplatesByCategory(category);
    }
  };

  const toggleSelectOne = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    const paged = templates.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);
    if (paged.every((t) => selectedIds.has(t.id))) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(paged.map((t) => t.id)));
    }
  };

  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    setDeleting(true);
    try {
      await deleteTemplates([...selectedIds]);
      toast.success(`已删除 ${selectedIds.size} 个模板`);
      setSelectedIds(new Set());
      setMultiSelectMode(false);
    } catch {
      toast.error('删除失败');
    } finally {
      setDeleting(false);
      setConfirmBatchDelete(false);
    }
  };

  const handleLaunch = useCallback(
    async (summary: TemplateSummary) => {
      const template = await getTemplate(summary.id);
      if (template) {
        setLaunchingTemplate(template);
      } else {
        toast.error('获取模板详情失败');
      }
    },
    [getTemplate],
  );

  const handleExecute = useCallback(
    async (variables: Record<string, string>) => {
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
    },
    [launchingTemplate, instantiateTemplate, startExecution, navigate],
  );

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">工作流模板</h1>
          <p className="text-sm text-muted-foreground mt-1">选择模板直接启动执行</p>
        </div>
        <div className="flex items-center gap-2">
          {multiSelectMode && (
            <>
              <button
                onClick={toggleSelectAll}
                className="flex items-center gap-1 px-3 py-1.5 text-sm border rounded-lg hover:bg-accent transition-colors"
              >
                {templates
                  .slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE)
                  .every((t) => selectedIds.has(t.id))
                  ? '取消全选'
                  : '全选'}
                <span className="text-xs text-muted-foreground ml-1">({selectedIds.size})</span>
              </button>
              <button
                onClick={() => setConfirmBatchDelete(true)}
                disabled={selectedIds.size === 0 || deleting}
                className="flex items-center gap-1 px-3 py-1.5 text-sm text-destructive border border-destructive/30 rounded-lg hover:bg-destructive/10 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Trash2 className="w-4 h-4" />
                {deleting ? '删除中...' : '删除所选'}
              </button>
            </>
          )}
          <button
            onClick={() => {
              setMultiSelectMode(!multiSelectMode);
              if (multiSelectMode) setSelectedIds(new Set());
            }}
            className={cn(
              'flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg border transition-colors',
              multiSelectMode
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-background text-muted-foreground border-border hover:bg-accent',
            )}
            title={multiSelectMode ? '退出多选' : '多选'}
          >
            <CheckSquare className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Category tabs */}
      <div className="px-4 border-b flex gap-2 overflow-x-auto">
        <button
          onClick={() => handleCategoryChange('all')}
          className={`px-3 py-2 text-sm rounded-lg transition-colors ${
            selectedCategory === null ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
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
          <div className="flex items-center justify-center h-64 text-destructive">{error}</div>
        )}
        {!loading && !error && templates.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <p>暂无模板</p>
          </div>
        )}
        {!loading &&
          !error &&
          templates.length > 0 &&
          (() => {
            const totalPages = Math.ceil(templates.length / PAGE_SIZE);
            const pagedTemplates = templates.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);
            return (
              <>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {pagedTemplates.map((template) => (
                    <TemplateCard
                      key={template.id}
                      template={template}
                      onLaunch={() => handleLaunch(template)}
                      multiSelectMode={multiSelectMode}
                      isSelected={selectedIds.has(template.id)}
                      onToggleSelect={() => toggleSelectOne(template.id)}
                    />
                  ))}
                </div>
                <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
              </>
            );
          })()}
      </div>

      {/* Launch Dialog */}
      {launchingTemplate && (
        <LaunchDialog
          template={launchingTemplate}
          onClose={() => setLaunchingTemplate(null)}
          onLaunch={handleExecute}
        />
      )}

      {/* Batch Delete Confirm */}
      <ConfirmModal
        isOpen={confirmBatchDelete}
        title="批量删除模板"
        message={`确定删除选中的 ${selectedIds.size} 个模板？此操作无法撤销。`}
        confirmText="删除"
        cancelText="取消"
        variant="danger"
        onConfirm={handleBatchDelete}
        onCancel={() => setConfirmBatchDelete(false)}
      />
    </div>
  );
}
