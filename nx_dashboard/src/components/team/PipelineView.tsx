import React from 'react';
import { motion } from 'motion/react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  usePipelineStore,
  PipelineStep,
  StepStatus,
  PipelineStatusType,
} from '../../stores/pipelineStore';

// --- Phase display config ---

const PHASE_LABELS: Record<string, string> = {
  requirements_analysis: '需求分析',
  architecture_design: '架构设计',
  project_init: '项目初始化',
  backend_dev: '后端开发',
  frontend_dev: '前端开发',
  api_integration: '接口联调',
  testing: '测试',
  documentation: '文档',
  packaging: '打包',
};

const STATUS_VARIANT: Record<StepStatus, 'secondary' | 'default' | 'success' | 'destructive' | 'warning' | 'outline'> = {
  pending: 'secondary',
  ready: 'default',
  running: 'default',
  completed: 'success',
  failed: 'destructive',
  skipped: 'outline',
  blocked: 'warning',
};

const STATUS_LABELS: Record<StepStatus, string> = {
  pending: '等待中',
  ready: '就绪',
  running: '执行中',
  completed: '已完成',
  failed: '失败',
  skipped: '已跳过',
  blocked: '被阻塞',
};

const PHASE_GROUPS = [
  { label: 'Phase 1 - 前置 (串行)', phases: ['requirements_analysis', 'architecture_design', 'project_init'] },
  { label: 'Phase 2 - 核心 (并行)', phases: ['backend_dev', 'frontend_dev'] },
  { label: 'Phase 3 - 收尾 (串行)', phases: ['api_integration', 'testing', 'documentation', 'packaging'] },
];

interface PipelineViewProps {
  projectId: string;
}

function StepCard({ step, pipelineId }: { step: PipelineStep; pipelineId: string }) {
  const retryStep = usePipelineStore((s) => s.retryStep);

  return (
    <div className="flex items-start gap-2 p-2 rounded border border-border bg-card">
      <Badge
        variant={STATUS_VARIANT[step.status]}
        className={`text-[10px] px-1.5 py-0 shrink-0 ${step.status === 'running' ? 'animate-pulse' : ''}`}
      >
        {STATUS_LABELS[step.status]}
      </Badge>
      <div className="flex-1 min-w-0">
        <span className="text-sm truncate block" title={step.instruction}>
          {step.instruction}
        </span>
        {step.status === 'failed' && step.output && (
          <p className="text-xs text-destructive mt-1 break-all">{step.output}</p>
        )}
      </div>
      {step.retry_count > 0 && (
        <span className="text-xs text-muted-foreground shrink-0">retry: {step.retry_count}</span>
      )}
      {(step.status === 'failed' || step.status === 'blocked') && (
        <Button size="sm" variant="destructive" className="h-6 text-xs px-2 shrink-0"
          onClick={() => retryStep(pipelineId, step.id)}>
          重试
        </Button>
      )}
    </div>
  );
}

function PhaseGroup({ phase, steps, pipelineId }: { phase: string; steps: PipelineStep[]; pipelineId: string }) {
  return (
    <div className="space-y-1">
      <h4 className="text-sm font-semibold text-foreground/70">{PHASE_LABELS[phase] || phase}</h4>
      {steps.map((step) => (
        <StepCard key={step.id} step={step} pipelineId={pipelineId} />
      ))}
    </div>
  );
}

function isTerminalStatus(status: PipelineStatusType): boolean {
  return status === 'completed' || status === 'failed' || status === 'idle';
}

export default function PipelineView({ projectId }: PipelineViewProps) {
  const {
    pipeline,
    loading,
    error,
    fetchPipeline,
    createPipeline,
    startPipeline,
    pausePipeline,
    resumePipeline,
    dispatchSteps,
    startPolling,
    stopPolling,
    clearError,
    reset,
  } = usePipelineStore();

  // Fetch pipeline on mount / projectId change
  React.useEffect(() => {
    if (projectId) fetchPipeline(projectId);
    return () => {
      reset();
    };
  }, [projectId, fetchPipeline, reset]);

  // Polling: start when pipeline is running, stop otherwise
  React.useEffect(() => {
    if (pipeline && pipeline.status === 'running') {
      startPolling(projectId);
    } else {
      stopPolling();
    }
    return () => {
      stopPolling();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pipeline?.status, projectId, startPolling, stopPolling]);

  if (!projectId) {
    return <div className="text-sm text-muted-foreground p-4">请先打开一个项目工作区</div>;
  }

  if (loading && !pipeline) {
    return <div className="text-sm text-muted-foreground p-4">加载 Pipeline...</div>;
  }

  if (error) {
    return (
      <div className="text-sm text-destructive p-4">
        错误: {error}
        <button className="ml-2 underline text-primary" onClick={clearError}>关闭</button>
      </div>
    );
  }

  if (!pipeline) {
    return (
      <div className="text-sm text-muted-foreground p-4 flex items-center gap-3">
        暂无 Pipeline
        <Button size="sm" onClick={() => createPipeline(projectId, '')}>创建 Pipeline</Button>
      </div>
    );
  }

  // Group steps by phase
  const stepsByPhase = new Map<string, PipelineStep[]>();
  for (const step of pipeline.steps) {
    const existing = stepsByPhase.get(step.phase) || [];
    stepsByPhase.set(step.phase, [...existing, step]);
  }

  const currentPhaseIndex = PHASE_GROUPS.findIndex((g) =>
    g.phases.includes(pipeline.current_phase),
  );

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-4 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-bold">Pipeline</h3>
          <p className="text-xs text-muted-foreground">
            阶段: {PHASE_LABELS[pipeline.current_phase] || pipeline.current_phase}
            {' | '}状态: {pipeline.status}
          </p>
        </div>
        <div className="flex gap-2">
          {pipeline.status === 'idle' && (
            <Button size="sm" onClick={() => startPipeline(pipeline.id)}>启动</Button>
          )}
          {pipeline.status === 'running' && (
            <>
              <Button size="sm" variant="destructive" onClick={() => dispatchSteps(pipeline.id)}>调度步骤</Button>
              <Button size="sm" variant="outline" onClick={() => pausePipeline(pipeline.id)}>暂停</Button>
            </>
          )}
          {pipeline.status === 'paused' && (
            <Button size="sm" onClick={() => resumePipeline(pipeline.id)}>恢复</Button>
          )}
          {isTerminalStatus(pipeline.status) && pipeline.status !== 'idle' && (
            <Button size="sm" variant="ghost" onClick={() => fetchPipeline(projectId)}>刷新</Button>
          )}
        </div>
      </div>

      {/* Progress bar */}
      <div>
        <div className="flex justify-between text-xs text-muted-foreground mb-1">
          <span>{pipeline.progress.completed_steps}/{pipeline.progress.total_steps} 步骤</span>
          <span>{pipeline.progress.progress_pct}%</span>
        </div>
        <div className="w-full bg-secondary rounded-full h-2">
          <motion.div
            className={`h-2 rounded-full ${pipeline.progress.failed_steps > 0 ? 'bg-destructive' : 'bg-primary'}`}
            initial={{ width: 0 }}
            animate={{ width: `${pipeline.progress.progress_pct}%` }}
            transition={{ duration: 0.4, ease: 'easeOut' }}
          />
        </div>
      </div>

      {/* Three-phase layout */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {PHASE_GROUPS.map((group, groupIdx) => {
          const isCurrentPhase = groupIdx === currentPhaseIndex;
          const isCompletedPhase = currentPhaseIndex > groupIdx;
          const borderClass = isCurrentPhase
            ? 'border-primary ring-2 ring-primary/20'
            : isCompletedPhase
              ? 'border-green-500/40 dark:border-green-700/40'
              : 'border-border';

          return (
            <div key={group.label} className={`space-y-2 p-3 rounded-lg border ${borderClass}`}>
              <h3 className={`text-sm font-bold ${isCompletedPhase ? 'text-green-600 dark:text-green-400 line-through' : 'text-foreground/80'}`}>
                {group.label}
                {isCurrentPhase && <span className="ml-2 text-xs text-primary">← 当前</span>}
                {isCompletedPhase && <span className="ml-2 text-xs text-green-500">✓</span>}
              </h3>
              {group.phases
                .filter((p) => stepsByPhase.has(p))
                .map((phase) => (
                  <PhaseGroup key={phase} phase={phase} steps={stepsByPhase.get(phase)!} pipelineId={pipeline.id} />
                ))}
            </div>
          );
        })}
      </div>
    </motion.div>
  );
}
