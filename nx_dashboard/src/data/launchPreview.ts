import {
  pipelineDurationHint,
  pipelineForWorkflow,
  pipelineLabelForWorkflow,
} from '@/data/workflowPipelines';
import type { StackProfile } from '@/data/stackProfile';

export interface LaunchPreviewData {
  workflowLabel: string;
  durationHint: string;
  stageCount: number;
  affectedPaths: string;
  estimatedCostUsd: number;
  codeLane: string;
  textLane: string;
  /** Phase B: stack-aware quality gate hint */
  qualityGateHint?: string;
}

function estimateAffectedPaths(workflowName: string, stack?: StackProfile): string {
  if (stack?.markerFiles.length) {
    return `${stack.markerFiles.join(' · ')} · src/`;
  }
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
      return 'src/ · 项目根';
  }
}

/** 规则估算 CLI 成本（非 LLM） */
export function estimateCliCostUsd(stageCount: number): number {
  const base = 0.08;
  return Math.round((base + stageCount * 0.04) * 100) / 100;
}

export function buildLaunchPreview(
  workflowName: string,
  stack?: StackProfile,
): LaunchPreviewData {
  const stages = pipelineForWorkflow(workflowName);
  const hasApiStage =
    workflowName === 'solo-dev' || workflowName === 'quick-fix' || workflowName === 'greenfield-mvp';

  const qualityGateHint = stack?.testCmd ?? stack?.buildCmd;

  return {
    workflowLabel: pipelineLabelForWorkflow(workflowName),
    durationHint: pipelineDurationHint(workflowName),
    stageCount: stages.length,
    affectedPaths: estimateAffectedPaths(workflowName, stack),
    estimatedCostUsd: estimateCliCostUsd(stages.length),
    codeLane: 'Claude CLI',
    textLane: hasApiStage ? 'API（摘要等）' : '—',
    qualityGateHint,
  };
}
