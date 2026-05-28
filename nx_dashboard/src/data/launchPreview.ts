import {
  pipelineDurationHint,
  pipelineForWorkflow,
  pipelineLabelForWorkflow,
} from '@/data/workflowPipelines';

export interface LaunchPreviewData {
  workflowLabel: string;
  durationHint: string;
  stageCount: number;
  affectedPaths: string;
  estimatedCostUsd: number;
  codeLane: string;
  textLane: string;
}

function estimateAffectedPaths(workflowName: string): string {
  switch (workflowName) {
    case 'greenfield-mvp':
      return 'package.json · src/ · README';
    case 'quick-fix':
      return 'src/ · 相关模块';
    case 'writing-plans':
      return 'docs/ · 计划文件';
    case 'dev-workflow':
      return 'src/ · tests/ · 多模块';
    default:
      return 'src/ · package.json';
  }
}

/** 规则估算 CLI 成本（非 LLM） */
export function estimateCliCostUsd(stageCount: number): number {
  const base = 0.08;
  return Math.round((base + stageCount * 0.04) * 100) / 100;
}

export function buildLaunchPreview(workflowName: string): LaunchPreviewData {
  const stages = pipelineForWorkflow(workflowName);
  const hasApiStage = workflowName === 'solo-dev' || workflowName === 'quick-fix' || workflowName === 'greenfield-mvp';

  return {
    workflowLabel: pipelineLabelForWorkflow(workflowName),
    durationHint: pipelineDurationHint(workflowName),
    stageCount: stages.length,
    affectedPaths: estimateAffectedPaths(workflowName),
    estimatedCostUsd: estimateCliCostUsd(stages.length),
    codeLane: 'Claude CLI',
    textLane: hasApiStage ? 'API（摘要等）' : '—',
  };
}
