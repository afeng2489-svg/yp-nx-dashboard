import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import {
  FileEdit,
  FileMinus,
  FilePlus,
  Loader2,
  PanelRightClose,
  Package,
  Terminal,
} from 'lucide-react';
import { useIsNarrow } from '@/hooks/useResponsive';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import type { ArtifactRecord, ArtifactSummary } from '@/api/client';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { ApprovalPanel } from '@/components/factory/ApprovalPanel';
import { TaskTimeline } from '@/components/factory/TaskTimeline';
import { ExecutionLaneBadge } from '@/components/factory/ExecutionLaneBadge';
import { isP5TaskTimelineEnabled } from '@/data/factoryFeatureFlags';
import { pipelineForWorkflow } from '@/data/workflowPipelines';
import {
  loadMergedArtifacts,
  workflowStageNamesFromResults,
} from '@/utils/executionLineage';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { WorkspacePipelineSummary } from '@/components/factory/WorkspacePipelineSummary';
import { cn } from '@/lib/utils';

function changeIcon(type: string) {
  if (type === 'added') return <FilePlus className="w-3.5 h-3.5 text-emerald-500" />;
  if (type === 'deleted') return <FileMinus className="w-3.5 h-3.5 text-red-500" />;
  return <FileEdit className="w-3.5 h-3.5 text-amber-500" />;
}

function statusLabel(status: string) {
  const map: Record<string, string> = {
    running: '运行中',
    completed: '完成',
    failed: '失败',
    paused: '暂停',
    pending: '等待',
    cancelled: '取消',
  };
  return map[status] ?? status;
}

export function ContextPanel() {
  const isNarrow = useIsNarrow(1024);
  const openDrawer = useFactoryDrawerStore((s) => s.open);
  const selectedExecutionId = useContextPanelStore((s) => s.selectedExecutionId);
  const close = useContextPanelStore((s) => s.close);
  const executions = useExecutionStore((s) => s.executions);
  const getExecution = useExecutionStore((s) => s.getExecution);
  const connectWebSocket = useExecutionStore((s) => s.connectWebSocket);
  const pendingPause = useExecutionStore((s) => s.pendingPause);
  const workflows = useWorkflowStore((s) => s.workflows);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);

  const [summary, setSummary] = useState<ArtifactSummary[]>([]);
  const [files, setFiles] = useState<ArtifactRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execution = useMemo(
    () => executions.find((e) => e.id === selectedExecutionId) ?? null,
    [executions, selectedExecutionId],
  );

  const workflowName = useMemo(() => {
    if (!execution) return '—';
    const wf = workflows.find((w) => w.id === execution.workflow_id);
    return wf?.name ?? execution.workflow_id.slice(0, 8);
  }, [execution, workflows]);

  useEffect(() => {
    if (!selectedExecutionId) return;
    if (execution?.status === 'running') connectWebSocket(selectedExecutionId);
  }, [selectedExecutionId, execution?.status, connectWebSocket]);

  useEffect(() => {
    if (!selectedExecutionId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);

    (async () => {
      try {
        const needsDetail =
          execution?.status === 'running' ||
          execution?.status === 'paused' ||
          Boolean(execution?.resumed_from) ||
          !execution?.stage_results?.length;
        if (needsDetail) {
          await getExecution(selectedExecutionId);
        }
        const liveExec =
          useExecutionStore.getState().executions.find((e) => e.id === selectedExecutionId) ??
          execution;
        if (!liveExec) return;
        const { summary: mergedSummary, files: mergedFiles } = await loadMergedArtifacts(liveExec);
        if (cancelled) return;
        setSummary(mergedSummary);
        setFiles(mergedFiles.slice(0, 12));
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : '加载产物失败');
          setSummary([]);
          setFiles([]);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [
    selectedExecutionId,
    execution?.status,
    execution?.resumed_from,
    execution?.stage_results?.length,
    getExecution,
  ]);

  if (!selectedExecutionId || !execution) return null;

  const totals = summary.reduce(
    (acc, s) => ({
      added: acc.added + s.added,
      modified: acc.modified + s.modified,
      deleted: acc.deleted + s.deleted,
    }),
    { added: 0, modified: 0, deleted: 0 },
  );
  const wf = workflows.find((w) => w.id === execution.workflow_id);
  const wfName = wf?.name ?? execution.workflow_id;
  // 进度以执行的真实阶段为准，避免依赖硬编码产线（未注册的工作流会算成 0/N）。
  // 总数优先用工作流定义的 stage_count，回退到注册产线长度。
  const completedNames = workflowStageNamesFromResults(execution);
  const pipelineLen = pipelineForWorkflow(wfName).length;
  const totalStages = wf?.stage_count && wf.stage_count > 0 ? wf.stage_count : pipelineLen;
  const stageCount =
    totalStages > 0 ? Math.min(completedNames.size, totalStages) : completedNames.size;
  const stageProgressPct =
    totalStages > 0 ? Math.min(100, Math.round((stageCount / totalStages) * 100)) : 0;
  const isApprovalPause =
    execution.status === 'paused' &&
    (execution.pending_pause?.pause_kind === 'approval' ||
      pendingPause?.execution_id === execution.id &&
        pendingPause.pause_kind === 'approval');
  const approvalQuestion =
    execution.pending_pause?.question ??
    (pendingPause?.execution_id === execution.id ? pendingPause.question : null);
  const approvalStage =
    execution.pending_pause?.stage_name ??
    (pendingPause?.execution_id === execution.id ? pendingPause.stage_name : '');

  return (
    <aside
      className={cn(
        'flex flex-col shrink-0 overflow-hidden bg-card border-border/60',
        isNarrow
          ? 'fixed inset-x-0 bottom-7 z-30 max-h-[55vh] border-t shadow-2xl animate-slide-up'
          : 'w-72 xl:w-80 border-l',
      )}
    >
      <div className="flex items-center justify-between gap-2 px-4 py-3 border-b border-border/50">
        <div className="min-w-0">
          <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Context</p>
          <p className="text-sm font-semibold truncate">{workflowName}</p>
          <ExecutionLaneBadge workflowName={wfName} stageName={execution.current_stage} />
        </div>
        <button
          type="button"
          onClick={close}
          className="p-1.5 rounded-md hover:bg-accent text-muted-foreground"
          title="关闭"
        >
          <PanelRightClose className="w-4 h-4" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <WorkspacePipelineSummary workspaceId={currentWorkspace?.id} />

        <div className="flex flex-wrap items-center gap-2 text-xs">
          <span
            className={cn(
              'px-2 py-0.5 rounded-full font-medium',
              execution.status === 'running' && 'bg-blue-500/15 text-blue-600',
              execution.status === 'completed' && 'bg-emerald-500/15 text-emerald-600',
              execution.status === 'failed' && 'bg-red-500/15 text-red-600',
            )}
          >
            {statusLabel(execution.status)}
          </span>
          {execution.trigger_source === 'factory' && (
            <span className="px-2 py-0.5 rounded-full bg-indigo-500/15 text-indigo-600">工厂</span>
          )}
          <span className="text-muted-foreground">{stageCount} 阶段</span>
        </div>

        {totalStages > 0 && (
          <div className="space-y-1">
            <div className="flex justify-between text-[10px] text-muted-foreground">
              <span>
                阶段 {stageCount}/{totalStages}
                {execution.current_stage ? ` · ${execution.current_stage}` : ''}
              </span>
              <span>{stageProgressPct}%</span>
            </div>
            <div className="h-1.5 rounded-full bg-muted overflow-hidden">
              <div
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${stageProgressPct}%` }}
              />
            </div>
          </div>
        )}

        {execution.resumed_from && (
          <p className="text-[11px] text-muted-foreground font-mono truncate" title={execution.resumed_from}>
            继承自 Run {execution.resumed_from.slice(0, 8)}…
          </p>
        )}

        {isP5TaskTimelineEnabled() && (
          <TaskTimeline
            execution={execution}
            userIntent={
              typeof execution.variables?.task === 'string'
                ? execution.variables.task
                : typeof execution.variables?.prompt === 'string'
                  ? execution.variables.prompt
                  : undefined
            }
          />
        )}

        <p className="text-[11px] text-muted-foreground font-mono truncate" title={execution.id}>
          Run {execution.id.slice(0, 8)}…
        </p>

        {loading && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
            <Loader2 className="w-4 h-4 animate-spin" />
            加载 diff 摘要…
          </div>
        )}

        {error && !loading && (
          <p className="text-xs text-destructive">{error}</p>
        )}

        {!loading && !error && (
          <>
            <div className="grid grid-cols-3 gap-2 text-center">
              <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 py-2">
                <p className="text-lg font-semibold text-emerald-600">{totals.added}</p>
                <p className="text-[10px] text-muted-foreground">新增</p>
              </div>
              <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 py-2">
                <p className="text-lg font-semibold text-amber-600">{totals.modified}</p>
                <p className="text-[10px] text-muted-foreground">修改</p>
              </div>
              <div className="rounded-lg border border-red-500/20 bg-red-500/5 py-2">
                <p className="text-lg font-semibold text-red-600">{totals.deleted}</p>
                <p className="text-[10px] text-muted-foreground">删除</p>
              </div>
            </div>

            {files.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <Package className="w-8 h-8 mx-auto mb-2 opacity-40" />
                <p className="text-xs">暂无文件变更</p>
                <p className="text-[11px] mt-1">Run 进行中或完成后会出现 diff</p>
              </div>
            ) : (
              <ul className="space-y-1">
                {files.map((f) => (
                  <li
                    key={f.id}
                    className="flex items-start gap-2 text-xs py-1.5 px-2 rounded-md hover:bg-accent/50"
                  >
                    <span className="mt-0.5 shrink-0">{changeIcon(f.change_type)}</span>
                    <span className="truncate font-mono" title={f.relative_path}>
                      {f.relative_path}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </div>

      {isApprovalPause && approvalQuestion && (
        <div className="px-4 pb-4 border-t border-border/50 pt-3">
          <ApprovalPanel
            executionId={execution.id}
            stageName={approvalStage}
            question={approvalQuestion}
            compact
          />
        </div>
      )}

      <div className="p-3 border-t border-border/50 space-y-2">
        <button
          type="button"
          onClick={() => openDrawer('terminal')}
          className="w-full flex items-center justify-center gap-1.5 text-xs py-1.5 rounded-md border border-border/50 hover:bg-accent/50"
        >
          <Terminal className="w-3.5 h-3.5" />
          在终端继续
        </button>
        <Link
          to={`/factory?tab=deliverables`}
          className="block text-center text-xs text-primary hover:underline"
        >
          查看全部交付物 →
        </Link>
      </div>
    </aside>
  );
}
