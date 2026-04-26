import React from 'react';
import { usePipelineStore, PipelineStep, StepStatus } from '../../stores/pipelineStore';

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

interface PipelineViewProps {
  projectId: string;
}

function StepCard({ step, pipelineId }: { step: PipelineStep; pipelineId: string }) {
  const retryStep = usePipelineStore(s => s.retryStep);

  return (
    <div className="flex items-center gap-2 p-2 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
      <span className={`px-2 py-0.5 rounded text-xs font-medium ${STATUS_COLORS[step.status]}`}>
        {STATUS_LABELS[step.status]}
      </span>
      <span className="text-sm flex-1 truncate" title={step.instruction}>
        {step.instruction}
      </span>
      {step.retry_count > 0 && (
        <span className="text-xs text-gray-400">retry: {step.retry_count}</span>
      )}
      {(step.status === 'failed' || step.status === 'blocked') && (
        <button
          className="text-xs px-2 py-0.5 rounded bg-red-100 text-red-700 hover:bg-red-200"
          onClick={() => retryStep(pipelineId, step.id)}
        >
          重试
        </button>
      )}
    </div>
  );
}

function PhaseGroup({ phase, steps, pipelineId }: { phase: string; steps: PipelineStep[]; pipelineId: string }) {
  return (
    <div className="space-y-1">
      <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
        {PHASE_LABELS[phase] || phase}
      </h4>
      {steps.map(step => (
        <StepCard key={step.id} step={step} pipelineId={pipelineId} />
      ))}
    </div>
  );
}

export default function PipelineView({ projectId }: PipelineViewProps) {
  const { pipeline, loading, error, fetchPipeline, startPipeline, pausePipeline, resumePipeline } = usePipelineStore();

  React.useEffect(() => {
    if (projectId) fetchPipeline(projectId);
  }, [projectId, fetchPipeline]);

  if (!projectId) {
    return <div className="text-sm text-gray-400 p-4">请先打开一个项目工作区</div>;
  }

  if (loading && !pipeline) {
    return <div className="text-sm text-gray-500 p-4">加载 Pipeline...</div>;
  }

  if (error) {
    return <div className="text-sm text-red-500 p-4">错误: {error}</div>;
  }

  if (!pipeline) {
    return <div className="text-sm text-gray-400 p-4">暂无 Pipeline</div>;
  }

  // Group steps by phase
  const stepsByPhase = new Map<string, PipelineStep[]>();
  for (const step of pipeline.steps) {
    const existing = stepsByPhase.get(step.phase) || [];
    stepsByPhase.set(step.phase, [...existing, step]);
  }

  // Order: Phase 1 → Phase 2 → Phase 3
  const phaseOrder = [
    'requirements_analysis', 'architecture_design', 'project_init',
    'backend_dev', 'frontend_dev',
    'api_integration', 'testing', 'documentation', 'packaging',
  ];

  const orderedPhases = phaseOrder.filter(p => stepsByPhase.has(p));

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
            <button
              className="px-3 py-1 text-sm rounded bg-yellow-500 text-white hover:bg-yellow-600"
              onClick={() => pausePipeline(pipeline.id)}
            >
              暂停
            </button>
          )}
          {pipeline.status === 'paused' && (
            <button
              className="px-3 py-1 text-sm rounded bg-green-500 text-white hover:bg-green-600"
              onClick={() => resumePipeline(pipeline.id)}
            >
              恢复
            </button>
          )}
        </div>
      </div>

      {/* Progress bar */}
      <div>
        <div className="flex justify-between text-xs text-gray-500 mb-1">
          <span>{pipeline.progress.completed_steps}/{pipeline.progress.total_steps} 步骤</span>
          <span>{pipeline.progress.progress_pct}%</span>
        </div>
        <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
          <div
            className="bg-blue-500 h-2 rounded-full transition-all duration-300"
            style={{ width: `${pipeline.progress.progress_pct}%` }}
          />
        </div>
      </div>

      {/* Three-phase layout */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Phase 1: Serial */}
        <div className="space-y-2 p-3 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 className="text-sm font-bold text-gray-600 dark:text-gray-400">Phase 1 - 前置 (串行)</h3>
          {orderedPhases
            .filter(p => ['requirements_analysis', 'architecture_design', 'project_init'].includes(p))
            .map(phase => (
              <PhaseGroup key={phase} phase={phase} steps={stepsByPhase.get(phase)!} pipelineId={pipeline.id} />
            ))}
        </div>

        {/* Phase 2: Parallel */}
        <div className="space-y-2 p-3 rounded-lg border border-blue-200 dark:border-blue-800">
          <h3 className="text-sm font-bold text-blue-600 dark:text-blue-400">Phase 2 - 核心 (并行)</h3>
          {orderedPhases
            .filter(p => ['backend_dev', 'frontend_dev'].includes(p))
            .map(phase => (
              <PhaseGroup key={phase} phase={phase} steps={stepsByPhase.get(phase)!} pipelineId={pipeline.id} />
            ))}
        </div>

        {/* Phase 3: Serial */}
        <div className="space-y-2 p-3 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 className="text-sm font-bold text-gray-600 dark:text-gray-400">Phase 3 - 收尾 (串行)</h3>
          {orderedPhases
            .filter(p => ['api_integration', 'testing', 'documentation', 'packaging'].includes(p))
            .map(phase => (
              <PhaseGroup key={phase} phase={phase} steps={stepsByPhase.get(phase)!} pipelineId={pipeline.id} />
            ))}
        </div>
      </div>
    </div>
  );
}
