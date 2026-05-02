import React from 'react';
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

const STATUS_COLORS: Record<StepStatus, string> = {
  pending: 'bg-gray-200 text-gray-600',
  ready: 'bg-blue-100 text-blue-700',
  running: 'bg-blue-500 text-white animate-pulse',
  completed: 'bg-green-500 text-white',
  failed: 'bg-red-500 text-white',
  skipped: 'bg-gray-300 text-gray-500',
  blocked: 'bg-orange-200 text-orange-700',
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
  {
    label: 'Phase 1 - 前置 (串行)',
    phases: ['requirements_analysis', 'architecture_design', 'project_init'],
    border: 'border-gray-200 dark:border-gray-700',
    titleColor: 'text-gray-600 dark:text-gray-400',
  },
  {
    label: 'Phase 2 - 核心 (并行)',
    phases: ['backend_dev', 'frontend_dev'],
    border: 'border-blue-200 dark:border-blue-800',
    titleColor: 'text-blue-600 dark:text-blue-400',
  },
  {
    label: 'Phase 3 - 收尾 (串行)',
    phases: ['api_integration', 'testing', 'documentation', 'packaging'],
    border: 'border-gray-200 dark:border-gray-700',
    titleColor: 'text-gray-600 dark:text-gray-400',
  },
];

interface PipelineViewProps {
  projectId: string;
}

function StepCard({ step, pipelineId }: { step: PipelineStep; pipelineId: string }) {
  const retryStep = usePipelineStore((s) => s.retryStep);

  return (
    <div className="flex items-start gap-2 p-2 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
      <span
        className={`px-2 py-0.5 rounded text-xs font-medium shrink-0 ${STATUS_COLORS[step.status]}`}
      >
        {STATUS_LABELS[step.status]}
      </span>
      <div className="flex-1 min-w-0">
        <span className="text-sm truncate block" title={step.instruction}>
          {step.instruction}
        </span>
        {step.status === 'failed' && step.output && (
          <p className="text-xs text-red-500 mt-1 break-all">{step.output}</p>
        )}
      </div>
      {step.retry_count > 0 && (
        <span className="text-xs text-gray-400 shrink-0">retry: {step.retry_count}</span>
      )}
      {(step.status === 'failed' || step.status === 'blocked') && (
        <button
          className="text-xs px-2 py-0.5 rounded bg-red-100 text-red-700 hover:bg-red-200 shrink-0"
          onClick={() => retryStep(pipelineId, step.id)}
        >
          重试
        </button>
      )}
    </div>
  );
}

function PhaseGroup({
  phase,
  steps,
  pipelineId,
}: {
  phase: string;
  steps: PipelineStep[];
  pipelineId: string;
}) {
  return (
    <div className="space-y-1">
      <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
        {PHASE_LABELS[phase] || phase}
      </h4>
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
  }, [pipeline?.status, projectId, startPolling, stopPolling]);

  if (!projectId) {
    return <div className="text-sm text-gray-400 p-4">请先打开一个项目工作区</div>;
  }

  if (loading && !pipeline) {
    return <div className="text-sm text-gray-500 p-4">加载 Pipeline...</div>;
  }

  if (error) {
    return (
      <div className="text-sm text-red-500 p-4">
        错误: {error}
        <button className="ml-2 underline text-blue-500" onClick={clearError}>
          关闭
        </button>
      </div>
    );
  }

  if (!pipeline) {
    return (
      <div className="text-sm text-gray-400 p-4">
        暂无 Pipeline
        <button
          className="ml-2 px-3 py-1 text-sm rounded bg-blue-500 text-white hover:bg-blue-600"
          onClick={() => createPipeline(projectId, '')}
        >
          创建 Pipeline
        </button>
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
    <div className="space-y-4 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-bold">Pipeline</h3>
          <p className="text-xs text-gray-500">
            阶段: {PHASE_LABELS[pipeline.current_phase] || pipeline.current_phase}
            {' | '}
            状态: {pipeline.status}
          </p>
        </div>
        <div className="flex gap-2">
          {pipeline.status === 'idle' && (
            <button
              className="px-3 py-1 text-sm rounded bg-blue-500 text-white hover:bg-blue-600"
              onClick={() => startPipeline(pipeline.id)}
            >
              启动
            </button>
          )}
          {pipeline.status === 'running' && (
            <>
              <button
                className="px-3 py-1 text-sm rounded bg-red-500 text-white hover:bg-red-600"
                onClick={() => dispatchSteps(pipeline.id)}
              >
                调度步骤
              </button>
              <button
                className="px-3 py-1 text-sm rounded bg-yellow-500 text-white hover:bg-yellow-600"
                onClick={() => pausePipeline(pipeline.id)}
              >
                暂停
              </button>
            </>
          )}
          {pipeline.status === 'paused' && (
            <button
              className="px-3 py-1 text-sm rounded bg-green-500 text-white hover:bg-green-600"
              onClick={() => resumePipeline(pipeline.id)}
            >
              恢复
            </button>
          )}
          {isTerminalStatus(pipeline.status) && pipeline.status !== 'idle' && (
            <button
              className="px-3 py-1 text-sm rounded bg-gray-400 text-white hover:bg-gray-500"
              onClick={() => fetchPipeline(projectId)}
            >
              刷新
            </button>
          )}
        </div>
      </div>

      {/* Progress bar */}
      <div>
        <div className="flex justify-between text-xs text-gray-500 mb-1">
          <span>
            {pipeline.progress.completed_steps}/{pipeline.progress.total_steps} 步骤
          </span>
          <span>{pipeline.progress.progress_pct}%</span>
        </div>
        <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
          <div
            className={`h-2 rounded-full transition-all duration-300 ${
              pipeline.progress.failed_steps > 0 ? 'bg-red-500' : 'bg-blue-500'
            }`}
            style={{ width: `${pipeline.progress.progress_pct}%` }}
          />
        </div>
      </div>

      {/* Three-phase layout */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {PHASE_GROUPS.map((group, groupIdx) => {
          const isCurrentPhase = groupIdx === currentPhaseIndex;
          const isCompletedPhase = currentPhaseIndex > groupIdx;
          const highlightBorder = isCurrentPhase
            ? 'border-red-400 dark:border-red-600 ring-2 ring-red-200 dark:ring-red-800'
            : isCompletedPhase
              ? 'border-green-300 dark:border-green-700'
              : group.border;

          return (
            <div key={group.label} className={`space-y-2 p-3 rounded-lg border ${highlightBorder}`}>
              <h3
                className={`text-sm font-bold ${isCompletedPhase ? 'text-green-600 dark:text-green-400 line-through' : group.titleColor}`}
              >
                {group.label}
                {isCurrentPhase && <span className="ml-2 text-xs text-red-500">← 当前</span>}
                {isCompletedPhase && <span className="ml-2 text-xs text-green-500">✓</span>}
              </h3>
              {group.phases
                .filter((p) => stepsByPhase.has(p))
                .map((phase) => (
                  <PhaseGroup
                    key={phase}
                    phase={phase}
                    steps={stepsByPhase.get(phase)!}
                    pipelineId={pipeline.id}
                  />
                ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
