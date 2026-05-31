import { useState } from 'react';
import { PauseCircle, X, Activity, AlertCircle, Globe, Network, Loader2, RotateCcw } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Execution } from '@/stores/executionStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { ArtifactsPanel } from '@/components/execution/ArtifactsPanel';
import { CanvasRunView } from '@/components/canvas/CanvasRunView';
import { usePreviewLauncher } from '@/lib/usePreviewLauncher';
import { nextStepsForRun, type RunNextStepAction } from '@/data/runNextSteps';
import { runFactoryQuickPrompt } from '@/services/factoryRun';
import { showError, showSuccess } from '@/lib/toast';
import { Button } from '@/components/ui/button';
import { STATUS_CONFIG } from './constants';
import { formatTime, formatDuration, useWorkflowName } from './utils';
import { StageResultCard } from './StageResultCard';
import { ExecutionLogs } from './ExecutionLogs';
import { GitTab } from './GitTab';

export interface ExecutionDetailModalProps {
  execution: Execution;
  onClose: () => void;
  onCancel: (id: string) => void;
}

export function ExecutionDetailModal({ execution, onClose, onCancel }: ExecutionDetailModalProps) {
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);
  const connectWebSocket = useExecutionStore((s) => s.connectWebSocket);
  const selectContextExecution = useContextPanelStore((s) => s.selectExecution);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const { launching, launch } = usePreviewLauncher();
  const [activeTab, setActiveTab] = useState<'stages' | 'logs' | 'artifacts' | 'git' | 'canvas'>(
    'stages',
  );
  const [expandedStages, setExpandedStages] = useState<Set<string>>(new Set());
  const [acting, setActing] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const workflowName = useWorkflowName();
  // 预览目录：优先用本次 Run 注入的 project_path，回退到当前工作区根目录。
  const previewPath =
    (typeof execution.variables?.project_path === 'string'
      ? execution.variables.project_path
      : undefined) ?? currentWorkspace?.root_path;
  const wfName = workflowName(execution.workflow_id);
  const failedSteps =
    execution.status === 'failed' ? nextStepsForRun(execution, wfName) : null;

  const runRetryAction = async (action: RunNextStepAction) => {
    if (action.kind !== 'retry' && action.kind !== 'run') return;
    setActing(true);
    setActionError(null);
    try {
      const result = await runFactoryQuickPrompt({
        prompt: action.prompt ?? '继续完成上次失败的任务',
        workflowName: action.workflowName,
        retryExecutionId: action.retryExecutionId,
        retryFromStage: action.retryFromStage,
        skipQualityGateForStage: action.skipQualityGateForStage,
      });
      if (result.ok) {
        showSuccess('已重新启动 Run');
        await fetchExecutions();
        if (result.executionId) {
          connectWebSocket(result.executionId);
          selectContextExecution(result.executionId);
        }
        onClose();
      } else {
        const msg = result.error ?? '重试失败';
        setActionError(msg);
        showError(msg);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : '重试失败';
      setActionError(msg);
      showError(msg);
    } finally {
      setActing(false);
    }
  };

  const config =
    STATUS_CONFIG[execution.status as keyof typeof STATUS_CONFIG] ?? STATUS_CONFIG.pending;
  const Icon = config.icon;

  const toggleStage = (stageName: string) => {
    setExpandedStages((prev) => {
      const next = new Set(prev);
      if (next.has(stageName)) {
        next.delete(stageName);
      } else {
        next.add(stageName);
      }
      return next;
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col animate-scale-in border border-border/50 overflow-hidden">
        {/* 弹窗头部 */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-muted/30">
          <div className="flex items-center gap-4">
            <div className={cn('p-2.5 rounded-xl bg-primary/10 text-primary')}>
              <Icon className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">执行详情</h2>
              <p className="text-sm text-muted-foreground font-mono">ID: {execution.id}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {previewPath && (
              <button
                onClick={() => launch(previewPath, execution.id)}
                disabled={launching}
                className="btn-primary px-4 py-2 text-sm flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                title="启动 dev server 预览本次生成的项目效果"
              >
                {launching ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Globe className="w-4 h-4" />
                )}
                {launching ? '启动预览中…' : '预览效果'}
              </button>
            )}
            {execution.status === 'running' && (
              <button
                onClick={() => onCancel(execution.id)}
                className="px-4 py-2 text-sm rounded-xl bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-colors"
              >
                取消执行
              </button>
            )}
            {execution.status === 'paused' && (
              <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-amber-500/10 border border-amber-500/30">
                <PauseCircle className="w-4 h-4 text-amber-500 animate-pulse" />
                <span className="text-sm text-amber-600 font-medium">等待用户输入</span>
              </div>
            )}
            <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* 执行信息 */}
        <div className="px-6 py-4 bg-muted/20 border-b border-border/50">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">工作流</p>
              <p className="font-semibold truncate" title={execution.workflow_id}>
                {workflowName(execution.workflow_id)}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">状态</p>
              <span
                className={cn(
                  'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium',
                  'bg-gradient-to-r ' + config.gradient,
                  'text-white shadow-md',
                )}
              >
                {config.label}
                {execution.resumed_from && (
                  <span className="ml-1.5 px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-600 font-medium">
                    已恢复
                  </span>
                )}
              </span>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">开始时间</p>
              <p className="font-medium text-sm">{formatTime(execution.started_at)}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">持续时间</p>
              <p className="font-medium text-sm">
                {formatDuration(execution.started_at, execution.finished_at)}
              </p>
            </div>
          </div>
        </div>

        {/* Tab 切换 */}
        <div className="flex border-b border-border/50">
          {(['stages', 'logs', 'artifacts', 'git', 'canvas'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={cn(
                'flex-1 px-4 py-3 text-sm font-medium transition-all relative',
                activeTab === tab
                  ? 'text-primary'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {tab === 'stages'
                ? '阶段结果'
                : tab === 'logs'
                  ? '执行日志'
                  : tab === 'artifacts'
                    ? '产物变更'
                    : tab === 'git'
                      ? 'Git'
                      : 'Canvas'}
              {activeTab === tab && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
              )}
            </button>
          ))}
        </div>

        {/* 内容区域 */}
        <div className="flex-1 overflow-auto p-6">
          {activeTab === 'stages' ? (
            <div className="space-y-3">
              {!execution.stage_results || execution.stage_results.length === 0 ? (
                <div className="text-center py-12">
                  <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-muted/60 flex items-center justify-center">
                    <Activity className="w-8 h-8 text-muted-foreground" />
                  </div>
                  <p className="text-muted-foreground">暂无阶段数据</p>
                </div>
              ) : (
                execution.stage_results?.map((result, idx) => (
                  <StageResultCard
                    key={idx}
                    result={result}
                    isExpanded={expandedStages.has(result.stage_name)}
                    onToggle={() => toggleStage(result.stage_name)}
                  />
                ))
              )}
            </div>
          ) : activeTab === 'logs' ? (
            <ExecutionLogs executionId={execution.id} />
          ) : activeTab === 'artifacts' ? (
            <ArtifactsPanel executionId={execution.id} />
          ) : activeTab === 'canvas' ? (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Network className="w-4 h-4" />
                只读产线 Canvas
              </div>
              <CanvasRunView executionId={execution.id} workflowId={execution.workflow_id} />
            </div>
          ) : (
            <GitTab executionId={execution.id} executionStatus={execution.status} />
          )}
        </div>

        {/* 错误信息 */}
        {(execution.error || (execution.status === 'failed' && failedSteps)) && (
          <div className="px-6 py-4 border-t border-border/50 bg-destructive/5 space-y-3">
            {execution.error && (
              <div className="flex items-start gap-3 p-3 rounded-xl bg-red-500/10 border border-red-500/20">
                <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
                <div className="min-w-0 flex-1">
                  <p className="font-medium text-red-600">执行错误</p>
                  <p className="text-sm mt-1 text-red-600/80 break-words">{execution.error}</p>
                </div>
              </div>
            )}
            {actionError && <p className="text-sm text-destructive px-1">{actionError}</p>}
            {failedSteps && (
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant="destructive"
                  className="gap-1.5"
                  disabled={acting}
                  onClick={() => void runRetryAction(failedSteps.primary)}
                >
                  {acting ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <RotateCcw className="h-3.5 w-3.5" />
                  )}
                  {failedSteps.primary.label}
                </Button>
                {failedSteps.secondary?.kind === 'run' && (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    disabled={acting}
                    onClick={() => void runRetryAction(failedSteps.secondary!)}
                  >
                    {failedSteps.secondary.label}
                  </Button>
                )}
                {failedSteps.tertiary?.kind === 'retry' && (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    disabled={acting}
                    onClick={() => void runRetryAction(failedSteps.tertiary!)}
                  >
                    {failedSteps.tertiary.label}
                  </Button>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
