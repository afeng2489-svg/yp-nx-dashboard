import { useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useExecutionStore, Execution } from '@/stores/executionStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import {
  Loader2,
  CheckCircle,
  XCircle,
  Clock,
  PauseCircle,
  AlertCircle,
  ChevronRight,
  Zap,
} from 'lucide-react';
import { pipelineForWorkflow } from '@/data/workflowPipelines';
import { cn } from '@/lib/utils';

const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    gradient: 'from-slate-400 to-gray-500',
    label: '等待',
    dot: 'bg-slate-400',
  },
  running: {
    icon: Loader2,
    gradient: 'from-blue-500 to-indigo-500',
    label: '运行',
    dot: 'bg-blue-500',
  },
  paused: {
    icon: PauseCircle,
    gradient: 'from-amber-400 to-orange-500',
    label: '暂停',
    dot: 'bg-amber-500',
  },
  completed: {
    icon: CheckCircle,
    gradient: 'from-emerald-500 to-green-500',
    label: '完成',
    dot: 'bg-emerald-500',
  },
  failed: { icon: XCircle, gradient: 'from-red-500 to-rose-500', label: '失败', dot: 'bg-red-500' },
  cancelled: {
    icon: XCircle,
    gradient: 'from-slate-400 to-gray-500',
    label: '取消',
    dot: 'bg-slate-400',
  },
  interrupted: {
    icon: AlertCircle,
    gradient: 'from-orange-500 to-amber-500',
    label: '中断',
    dot: 'bg-orange-500',
  },
} as const;

function formatDuration(startedAt?: string, finishedAt?: string): string {
  if (!startedAt) return '-';
  const start = new Date(startedAt).getTime();
  const end = finishedAt ? new Date(finishedAt).getTime() : Date.now();
  const secs = Math.floor((end - start) / 1000);
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

function useWorkflowName() {
  const workflows = useWorkflowStore((s) => s.workflows);
  return (workflowId: string): string => {
    const found = workflows.find((w) => w.id === workflowId);
    if (found) return found.name;
    return workflowId.length > 12 ? `${workflowId.slice(0, 8)}...` : workflowId;
  };
}

function ActiveExecutionRow({ execution, onClick }: { execution: Execution; onClick: () => void }) {
  const config = STATUS_CONFIG[execution.status] ?? STATUS_CONFIG.pending;
  const workflowName = useWorkflowName();
  const workflows = useWorkflowStore((s) => s.workflows);
  const wf = workflows.find((w) => w.id === execution.workflow_id);
  const wfKey = wf?.name ?? execution.workflow_id;
  const pipeline = pipelineForWorkflow(wfKey);
  const completedNames = new Set((execution.stage_results ?? []).map((s) => s.stage_name));
  const doneStages = pipeline.filter((s) => completedNames.has(s.name)).length;
  const totalStages = pipeline.length;
  const currentStage =
    execution.current_stage ??
    pipeline.find((s) => !completedNames.has(s.name))?.name;
  const runningLabel = currentStage ?? '启动中…';

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-4 p-4 rounded-xl transition-all duration-200',
        'bg-gradient-to-r from-card to-accent/30 border border-border/50',
        'hover:shadow-md hover:shadow-primary/5 hover:border-primary/20 hover:-translate-y-0.5',
        'text-left group',
      )}
    >
      <div
        className={cn(
          'w-2.5 h-2.5 rounded-full shrink-0',
          config.dot,
          execution.status === 'running' && 'animate-pulse',
        )}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="font-medium truncate group-hover:text-indigo-600 transition-colors">
            {workflowName(execution.workflow_id)}
          </p>
          <span
            className={cn(
              'px-2 py-0.5 rounded-full text-[10px] font-medium text-white',
              'bg-gradient-to-r ' + config.gradient,
            )}
          >
            {config.label}
          </span>
          {execution.trigger_source === 'factory' && (
            <span className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-indigo-500/15 text-indigo-600 dark:text-indigo-300 border border-indigo-500/30">
              工厂
            </span>
          )}
        </div>
        <div className="flex items-center gap-3 mt-1.5">
          {execution.status === 'running' && (
            <span className="text-xs text-blue-500 animate-pulse">{runningLabel}</span>
          )}
          {execution.status === 'paused' && execution.pending_pause?.stage_name && (
            <span className="text-xs text-amber-600">
              等待 · {execution.pending_pause.stage_name}
            </span>
          )}
          <span className="text-xs text-muted-foreground">
            {doneStages > 0 ? `${doneStages}/${totalStages} 阶段` : `${totalStages} 阶段`}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatDuration(execution.started_at, execution.finished_at)}
          </span>
        </div>
      </div>
      <ChevronRight className="w-4 h-4 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all shrink-0" />
    </button>
  );
}

function PipelineGanttBar({ execution }: { execution: Execution }) {
  const stages = execution.stage_results ?? [];
  if (stages.length === 0) return null;

  const completedCount = stages.length;
  const totalEstimate = Math.max(completedCount + 1, 2);
  const segmentWidth = 100 / totalEstimate;

  return (
    <div className="flex items-center gap-1 h-6">
      {stages.map((stage) => {
        const qg = stage.quality_gate_result;
        const passed = qg ? qg.passed : true;
        return (
          <div
            key={stage.stage_name}
            className={cn(
              'h-full rounded flex items-center justify-center text-[10px] font-medium text-white transition-all',
              passed
                ? 'bg-gradient-to-r from-emerald-500 to-green-500'
                : 'bg-gradient-to-r from-red-500 to-rose-500',
            )}
            style={{ width: `${segmentWidth}%`, minWidth: 24 }}
            title={`${stage.stage_name}${qg ? (qg.passed ? ' ✓' : ' ✗') : ''}`}
          >
            <span className="truncate px-1">{stage.stage_name.slice(0, 4)}</span>
          </div>
        );
      })}
      {execution.status === 'running' && (
        <div
          className="h-full rounded bg-gradient-to-r from-blue-500 to-indigo-500 flex items-center justify-center text-[10px] font-medium text-white animate-pulse"
          style={{ width: `${segmentWidth}%`, minWidth: 24 }}
          title={execution.current_stage ?? '进行中'}
        >
          {(execution.current_stage ?? '…').slice(0, 4)}
        </div>
      )}
    </div>
  );
}

export function ActiveExecutionsPanel({ variant = 'classic' }: { variant?: 'classic' | 'flat' }) {
  const navigate = useNavigate();
  const executions = useExecutionStore((s) => s.executions);
  const connectWebSocket = useExecutionStore((s) => s.connectWebSocket);
  const selectContextExecution = useContextPanelStore((s) => s.selectExecution);
  const getWorkflowName = useWorkflowName();
  const activeExecutions = useMemo(
    () =>
      executions.filter(
        (e) => e.status === 'running' || e.status === 'paused' || e.status === 'pending',
      ),
    [executions],
  );

  const recentCompleted = useMemo(
    () =>
      executions
        .filter((e) => e.status === 'completed' || e.status === 'failed')
        .slice(0, 3),
    [executions],
  );

  const hasActive = activeExecutions.length > 0;
  const hasAny = hasActive || recentCompleted.length > 0;
  const prevActiveCount = useRef(activeExecutions.length);

  const wsTargetKey = useMemo(
    () =>
      activeExecutions
        .filter((e) => e.status === 'running' || e.status === 'paused')
        .map((e) => e.id)
        .sort()
        .join(','),
    [activeExecutions],
  );

  useEffect(() => {
    if (!wsTargetKey) return;
    for (const id of wsTargetKey.split(',')) {
      connectWebSocket(id);
    }
  }, [wsTargetKey, connectWebSocket]);

  useEffect(() => {
    prevActiveCount.current = activeExecutions.length;
  }, [activeExecutions.length]);

  if (variant === 'flat') {
    if (!hasAny) {
      return (
        <p className="text-xs text-muted-foreground text-center py-2">暂无进行中的 Run</p>
      );
    }
    return (
      <div className="space-y-1">
        {activeExecutions.map((execution) => {
          const config = STATUS_CONFIG[execution.status] ?? STATUS_CONFIG.pending;
          return (
            <button
              key={execution.id}
              type="button"
              onClick={() => selectContextExecution(execution.id)}
              className="w-full flex items-center gap-2 py-2 text-sm text-left text-muted-foreground hover:text-foreground transition-colors"
            >
              <div
                className={cn(
                  'w-1.5 h-1.5 rounded-full shrink-0',
                  config.dot,
                  execution.status === 'running' && 'animate-pulse',
                )}
              />
              <span className="truncate flex-1">{getWorkflowName(execution.workflow_id)}</span>
              <span className="text-xs shrink-0">{config.label}</span>
            </button>
          );
        })}
      </div>
    );
  }

  if (!hasAny) {
    return (
      <div className="bg-card rounded-2xl border border-border/50 p-6 shadow-sm">
        <div className="flex items-center gap-3 mb-5">
          <div className="p-2 rounded-xl bg-gradient-to-br from-blue-500/10 to-indigo-500/10">
            <Zap className="w-5 h-5 text-blue-500" />
          </div>
          <h2 className="text-lg font-semibold">生产线状态</h2>
        </div>
        <div className="text-center py-8">
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-muted to-accent flex items-center justify-center">
            <Zap className="w-8 h-8 text-muted-foreground" />
          </div>
          <p className="text-muted-foreground">暂无活跃执行</p>
          <p className="text-sm text-muted-foreground/70 mt-1">
            执行工作流后，实时进度将显示在这里
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card rounded-2xl border border-border/50 p-6 shadow-sm space-y-5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-xl bg-gradient-to-br from-blue-500/10 to-indigo-500/10">
            <Zap className="w-5 h-5 text-blue-500" />
          </div>
          <h2 className="text-lg font-semibold">生产线状态</h2>
          {hasActive && (
            <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 text-xs font-medium animate-pulse">
              {activeExecutions.length} 活跃
            </span>
          )}
        </div>
        <button
          onClick={() => navigate('/executions')}
          className="text-sm text-primary hover:text-primary/80 transition-colors font-medium"
        >
          查看全部 →
        </button>
      </div>

      {hasActive && (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">
            活跃执行
          </p>
          {activeExecutions.map((execution) => (
            <div key={execution.id}>
              <ActiveExecutionRow
                execution={execution}
                onClick={() => selectContextExecution(execution.id)}
              />
              {execution.stage_results && execution.stage_results.length > 0 && (
                <div className="mt-1 px-4">
                  <PipelineGanttBar execution={execution} />
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {recentCompleted.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">
            最近完成
          </p>
          {recentCompleted.map((execution) => {
            const config = STATUS_CONFIG[execution.status] ?? STATUS_CONFIG.pending;
            return (
              <button
                key={execution.id}
                onClick={() => selectContextExecution(execution.id)}
                className="w-full flex items-center gap-3 p-3 rounded-xl bg-gradient-to-r from-card to-accent/20 border border-border/30 hover:border-primary/20 transition-all text-left group"
              >
                <div className={cn('w-2 h-2 rounded-full shrink-0', config.dot)} />
                <span className="text-sm truncate group-hover:text-indigo-600 transition-colors flex-1">
                  {getWorkflowName(execution.workflow_id)}
                </span>
                <span
                  className={cn(
                    'text-[10px] px-2 py-0.5 rounded-full font-medium text-white bg-gradient-to-r ' +
                      config.gradient,
                  )}
                >
                  {config.label}
                </span>
                <span className="text-xs text-muted-foreground">
                  {formatDuration(execution.started_at, execution.finished_at)}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
