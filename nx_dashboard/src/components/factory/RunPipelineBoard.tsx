import { useMemo } from 'react';
import {
  Check,
  Circle,
  Loader2,
  ShieldAlert,
  User,
  Users,
} from 'lucide-react';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useTeamStore } from '@/stores/teamStore';
import {
  formatPipelineStageSummary,
  inferCurrentStageName,
  nextGateHint,
  pipelineForWorkflow,
  pipelineLabelForWorkflow,
  resolveStageStates,
  type PipelineStageDef,
  type StageVisualState,
} from '@/data/workflowPipelines';
import { cn } from '@/lib/utils';

function stageIcon(state: StageVisualState, kind: PipelineStageDef['kind']) {
  if (state === 'done') return Check;
  if (state === 'waiting') return ShieldAlert;
  if (state === 'active') return Loader2;
  if (kind === 'approval') return User;
  return Circle;
}

function stageStyles(state: StageVisualState) {
  switch (state) {
    case 'done':
      return {
        node: 'bg-emerald-500/15 border-emerald-500/40 text-emerald-700 dark:text-emerald-300',
        label: 'text-foreground',
      };
    case 'active':
      return {
        node: 'bg-primary/15 border-primary ring-2 ring-primary/30 text-primary',
        label: 'text-foreground font-medium',
      };
    case 'waiting':
      return {
        node: 'bg-amber-500/15 border-amber-500/50 text-amber-700 dark:text-amber-300 animate-pulse',
        label: 'text-amber-800 dark:text-amber-200 font-medium',
      };
    case 'failed':
      return {
        node: 'bg-destructive/15 border-destructive/50 text-destructive',
        label: 'text-destructive',
      };
    default:
      return {
        node: 'bg-muted/40 border-border text-muted-foreground',
        label: 'text-muted-foreground',
      };
  }
}

function PipelineStageNode({
  stage,
  state,
}: {
  stage: PipelineStageDef;
  state: StageVisualState;
}) {
  const styles = stageStyles(state);
  const Icon = stageIcon(state, stage.kind);
  const spinning = state === 'active';

  return (
    <div className="flex flex-col items-center w-full">
      <div
        className={cn(
          'flex h-10 w-10 items-center justify-center rounded-full border-2 shrink-0 transition-colors',
          styles.node,
        )}
        title={stage.gateLabel ? `${stage.name} · ${stage.gateLabel}` : stage.name}
      >
        <Icon className={cn('h-4 w-4', spinning && 'animate-spin')} />
      </div>
      <p className={cn('mt-2 text-xs text-center truncate w-full px-0.5', styles.label)}>{stage.name}</p>
      <p className="text-[10px] text-muted-foreground text-center truncate w-full px-0.5">{stage.role}</p>
      {stage.gateLabel && state !== 'pending' && (
        <p className="text-[10px] text-muted-foreground/80 mt-0.5 truncate w-full text-center">
          {stage.gateLabel}
        </p>
      )}
    </div>
  );
}

function RunPipelineTrack({ execution, workflowName }: { execution: Execution; workflowName: string }) {
  const pipeline = pipelineForWorkflow(workflowName);
  const completed = (execution.stage_results ?? []).map((s) => s.stage_name);
  const current = inferCurrentStageName(
    execution.current_stage,
    execution.pending_pause?.stage_name,
    execution.stage_results,
  );

  const states = useMemo(
    () => resolveStageStates(pipeline, completed, current, execution.status),
    [pipeline, completed, current, execution.status],
  );

  const hint = nextGateHint(pipeline, states, current);
  const taskPreview =
    typeof execution.variables?.task === 'string'
      ? execution.variables.task
      : typeof execution.variables?.prompt === 'string'
        ? execution.variables.prompt
        : null;

  return (
    <div className="space-y-4">
      {taskPreview && (
        <p className="text-sm text-muted-foreground line-clamp-2">
          <span className="text-foreground font-medium">本批次：</span>
          {taskPreview}
        </p>
      )}

      <div className="flex items-start w-full gap-0">
        {pipeline.map((stage, i) => (
          <div key={stage.name} className="flex items-start flex-1 min-w-0">
            {i > 0 && (
              <div
                className={cn(
                  'h-0.5 flex-1 mt-5 min-w-[8px]',
                  (states[i - 1] ?? 'pending') === 'done' ? 'bg-emerald-500/50' : 'bg-border',
                )}
              />
            )}
            <div className="flex flex-col items-center shrink-0 w-[72px] sm:w-[80px]">
              <PipelineStageNode stage={stage} state={states[i] ?? 'pending'} />
            </div>
            {i < pipeline.length - 1 && (
              <div
                className={cn(
                  'h-0.5 flex-1 mt-5 min-w-[8px]',
                  (states[i] ?? 'pending') === 'done' ? 'bg-emerald-500/50' : 'bg-border',
                )}
              />
            )}
          </div>
        ))}
      </div>

      {hint && (
        <div
          className={cn(
            'rounded-lg border px-3 py-2 text-sm',
            execution.status === 'paused'
              ? 'border-amber-500/40 bg-amber-500/10 text-amber-900 dark:text-amber-100'
              : 'border-border bg-muted/30 text-muted-foreground',
          )}
        >
          {hint}
        </div>
      )}
    </div>
  );
}

function IdlePipelinePreview({ workflowName }: { workflowName: string }) {
  const pipeline = pipelineForWorkflow(workflowName);
  const summary = formatPipelineStageSummary(workflowName);

  return (
    <div className="text-center py-6 px-4">
      <Users className="h-10 w-10 mx-auto text-muted-foreground/40 mb-3" />
      <p className="text-sm font-medium text-foreground">产线待命 · {pipelineLabelForWorkflow(workflowName)}</p>
      <p className="text-xs text-muted-foreground mt-2 max-w-md mx-auto leading-relaxed">
        启动后将依次执行：
        <span className="text-foreground/80"> {summary}</span>
      </p>
      <div className="flex flex-wrap justify-center gap-1.5 mt-4 max-w-lg mx-auto">
        {pipeline.map((stage) => (
          <span
            key={stage.name}
            className="text-[10px] px-2 py-0.5 rounded-full bg-muted/60 border border-border/40 text-muted-foreground"
          >
            {stage.name}
          </span>
        ))}
      </div>
    </div>
  );
}

export interface RunPipelineBoardProps {
  /** 无活跃 Run 时预览的产线（来自意图输入 / 推荐） */
  previewWorkflowName?: string;
}

/** 工厂台 · 产线看板（AF-12：随 workflow 切换） */
export function RunPipelineBoard({ previewWorkflowName = 'solo-dev' }: RunPipelineBoardProps) {
  const executions = useExecutionStore((s) => s.executions);
  const workflows = useWorkflowStore((s) => s.workflows);
  const contextId = useContextPanelStore((s) => s.selectedExecutionId);
  const currentTeam = useTeamStore((s) => s.currentTeam);

  const hero = useMemo(() => {
    const active = executions.filter(
      (e) => e.status === 'running' || e.status === 'paused' || e.status === 'pending',
    );
    if (contextId) {
      const selected = active.find((e) => e.id === contextId);
      if (selected) return selected;
    }
    const factory = active.find((e) => e.trigger_source === 'factory');
    return factory ?? active[0] ?? null;
  }, [executions, contextId]);

  const workflowName = useMemo(() => {
    if (hero) {
      const w = workflows.find((x) => x.id === hero.workflow_id);
      return w?.name ?? 'solo-dev';
    }
    return previewWorkflowName || 'solo-dev';
  }, [hero, workflows, previewWorkflowName]);

  const displayLabel = pipelineLabelForWorkflow(workflowName);

  return (
    <section
      className="rounded-2xl border border-border/60 bg-card shadow-sm overflow-hidden"
      data-testid="run-pipeline-board"
    >
      <div className="flex items-start justify-between gap-3 px-5 py-4 border-b border-border/50 bg-muted/20">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <Users className="h-4 w-4 text-primary shrink-0" />
            <h3 className="text-sm font-semibold">虚拟团队 · {displayLabel}</h3>
            {hero && (
              <span
                className={cn(
                  'text-[10px] px-2 py-0.5 rounded-full font-medium',
                  hero.status === 'paused'
                    ? 'bg-amber-500/15 text-amber-700 dark:text-amber-300'
                    : 'bg-primary/10 text-primary',
                )}
              >
                {hero.status === 'paused' ? '等待你' : '运行中'}
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {currentTeam?.name ? `车间：${currentTeam.name} · ` : ''}
            质量门与审批自动卡点，你只需在关键节点拍板
          </p>
        </div>
        {hero && (
          <code className="text-[10px] text-muted-foreground font-mono shrink-0 hidden sm:block">
            {hero.id.slice(0, 8)}…
          </code>
        )}
      </div>

      <div className="px-5 py-5">
        {hero ? (
          <RunPipelineTrack execution={hero} workflowName={workflowName} />
        ) : (
          <IdlePipelinePreview workflowName={workflowName} />
        )}
      </div>
    </section>
  );
}
