/** 工厂产线阶段 — 用于 RunPipelineBoard（用户可见阶段名，非 workflow id） */
export type PipelineStageKind = 'agent' | 'approval' | 'gate';

export interface PipelineStageDef {
  name: string;
  role: string;
  kind: PipelineStageKind;
  /** 质量门 / 门控说明（展示用） */
  gateLabel?: string;
}

export const SOLO_DEV_PIPELINE: PipelineStageDef[] = [
  { name: '规划', role: '全栈工程师', kind: 'agent' },
  { name: '实现', role: '全栈工程师', kind: 'gate', gateLabel: '编译通过' },
  { name: '自测', role: '全栈工程师', kind: 'gate', gateLabel: '测试通过' },
  { name: '交付审批', role: '你 · 厂长', kind: 'approval', gateLabel: '人工批准' },
  { name: '审查', role: '全栈工程师', kind: 'agent' },
  { name: '交付摘要', role: '交付摘要员', kind: 'agent' },
];

export const QUICK_FIX_PIPELINE: PipelineStageDef[] = [
  { name: '分析', role: 'Bug 分析员', kind: 'agent' },
  { name: '修复', role: '修复工程师', kind: 'agent' },
  { name: '验证', role: '质量门', kind: 'gate', gateLabel: '测试通过' },
  { name: '交付审批', role: '你 · 厂长', kind: 'approval', gateLabel: '人工批准' },
  { name: '交付摘要', role: '交付摘要员', kind: 'agent' },
];

export const DEV_WORKFLOW_PIPELINE: PipelineStageDef[] = [
  { name: '规划', role: '架构师', kind: 'agent' },
  { name: '实现', role: '开发工程师', kind: 'gate', gateLabel: '编译通过' },
  { name: '审查与测试', role: '审查 ∥ 测试', kind: 'gate', gateLabel: '审查+测试' },
  { name: '交付审批', role: '你 · 厂长', kind: 'approval', gateLabel: '人工批准' },
  { name: '交付摘要', role: '交付摘要员', kind: 'agent' },
];

export const WRITING_PLANS_PIPELINE: PipelineStageDef[] = [
  { name: '范围检查', role: '架构师', kind: 'agent' },
  { name: '编写计划', role: '计划员', kind: 'agent' },
  { name: '自我审查', role: '审查员', kind: 'agent' },
];

export const REVIEW_CYCLE_PIPELINE: PipelineStageDef[] = [
  { name: '代码审查', role: '审查员', kind: 'agent' },
  { name: '改进建议', role: '审查员', kind: 'agent' },
];

export const INVESTIGATE_PIPELINE: PipelineStageDef[] = [
  { name: '调查', role: '调查员', kind: 'agent' },
  { name: '根因报告', role: '调查员', kind: 'agent' },
];

/** AF-16 预览 — 与 greenfield-mvp.yaml 对齐 */
export const GREENFIELD_PREVIEW_PIPELINE: PipelineStageDef[] = [
  { name: '需求澄清', role: '产品经理', kind: 'agent' },
  { name: '技术选型', role: '架构师', kind: 'agent' },
  { name: '脚手架', role: '脚手架工程师', kind: 'agent' },
  { name: '核心骨架', role: '全栈工程师', kind: 'gate', gateLabel: '可编译' },
  { name: '环境验证', role: '全栈工程师', kind: 'gate', gateLabel: '可运行' },
  { name: '交付审批', role: '你 · 厂长', kind: 'approval', gateLabel: '人工批准' },
  { name: '交付摘要', role: '交付摘要员', kind: 'agent' },
];

const PIPELINE_REGISTRY: Record<string, PipelineStageDef[]> = {
  'solo-dev': SOLO_DEV_PIPELINE,
  'quick-fix': QUICK_FIX_PIPELINE,
  'dev-workflow': DEV_WORKFLOW_PIPELINE,
  'writing-plans': WRITING_PLANS_PIPELINE,
  'review-cycle': REVIEW_CYCLE_PIPELINE,
  investigate: INVESTIGATE_PIPELINE,
  'greenfield-mvp': GREENFIELD_PREVIEW_PIPELINE,
};

/** 对用户展示的中文产线名（禁止直接暴露 workflow id） */
const PIPELINE_LABELS: Record<string, string> = {
  'solo-dev': '一人全栈',
  'quick-fix': '快速修复',
  'dev-workflow': '全栈四人组',
  'writing-plans': '实施计划',
  'review-cycle': '代码审查',
  investigate: '技术调研',
  'greenfield-mvp': '从零搭项目',
  'ui-ux-design': 'UI 设计',
  'landing-page': '落地页生成',
  'nav-site': '导航站生成',
};

export function pipelineForWorkflow(workflowName?: string): PipelineStageDef[] {
  const key = workflowName?.trim() || 'solo-dev';
  return PIPELINE_REGISTRY[key] ?? SOLO_DEV_PIPELINE;
}

/** 该工作流是否有精心维护的产线注册表（有则用注册表的中文阶段名/角色） */
export function hasRegisteredPipeline(workflowName?: string): boolean {
  const key = workflowName?.trim();
  return Boolean(key && PIPELINE_REGISTRY[key]);
}

type RawStage = {
  name?: string;
  stage_type?: string;
  question?: string;
  quality_gate?: unknown;
};

/** 用工作流定义里的真实 stages 构建产线（未注册的工作流，如 landing-page/nav-site） */
export function mapStagesToPipeline(stages: RawStage[]): PipelineStageDef[] {
  return stages
    .filter((s): s is RawStage & { name: string } => Boolean(s.name) && !s.name!.startsWith('agent:'))
    .map((s) => {
      const isApproval = s.stage_type === 'approval' || Boolean(s.question);
      const isGate = !isApproval && (s.stage_type === 'quality_gate' || s.quality_gate != null);
      const kind: PipelineStageKind = isApproval ? 'approval' : isGate ? 'gate' : 'agent';
      return {
        name: s.name,
        role: isApproval ? '你 · 厂长' : '',
        kind,
        gateLabel: isApproval ? '人工批准' : isGate ? '质量门' : undefined,
      };
    });
}

/** 兜底：从执行结果（已完成阶段 + 当前阶段）推断真实产线，保证名字与进度一致 */
export function deriveStagesFromExecution(execution: {
  stage_results?: { stage_name: string; quality_gate_result?: unknown }[];
  current_stage?: string;
}): PipelineStageDef[] {
  const seen = new Set<string>();
  const out: PipelineStageDef[] = [];
  const push = (name: string | undefined, gate: boolean) => {
    if (!name || name.startsWith('agent:') || seen.has(name)) return;
    seen.add(name);
    out.push({ name, role: '', kind: gate ? 'gate' : 'agent' });
  };
  for (const sr of execution.stage_results ?? []) {
    push(sr.stage_name, sr.quality_gate_result != null);
  }
  push(execution.current_stage, false);
  return out;
}

export function pipelineLabelForWorkflow(workflowName?: string): string {
  const key = workflowName?.trim() || 'solo-dev';
  return PIPELINE_LABELS[key] ?? '一人全栈';
}

/** 展示用时长 hint（规则，非 LLM） */
export function pipelineDurationHint(workflowName?: string): string {
  const n = pipelineForWorkflow(workflowName).length;
  if (n <= 3) return '约 5–15 分钟';
  if (n <= 5) return '约 10–20 分钟';
  return '约 15–30 分钟';
}

export function formatPipelineStageSummary(workflowName?: string): string {
  const stages = pipelineForWorkflow(workflowName);
  return stages.map((s) => s.name).join(' → ');
}

export type StageVisualState = 'done' | 'active' | 'waiting' | 'pending' | 'failed';

export function resolveStageStates(
  pipeline: PipelineStageDef[],
  completedNames: string[],
  currentStage?: string,
  executionStatus?: string,
): StageVisualState[] {
  const completed = new Set(completedNames);
  const currentIdx = currentStage
    ? pipeline.findIndex((s) => s.name === currentStage)
    : Math.min(completed.size, pipeline.length - 1);

  return pipeline.map((stage, i) => {
    if (completed.has(stage.name)) return 'done';

    if (executionStatus === 'failed' && stage.name === currentStage) return 'failed';

    const isCurrent = stage.name === currentStage || (currentStage === undefined && i === currentIdx);

    if (isCurrent || i === currentIdx) {
      if (executionStatus === 'paused') return 'waiting';
      if (executionStatus === 'running' || executionStatus === 'pending') return 'active';
    }

    return 'pending';
  });
}

/** 从 execution 推断当前阶段名 */
export function inferCurrentStageName(
  currentStage?: string,
  pendingPauseStage?: string,
  stageResults?: { stage_name: string }[],
): string | undefined {
  if (pendingPauseStage) return pendingPauseStage;
  if (currentStage) return currentStage;
  if (stageResults?.length) {
    return stageResults[stageResults.length - 1]?.stage_name;
  }
  return undefined;
}

export function nextGateHint(
  pipeline: PipelineStageDef[],
  states: StageVisualState[],
  currentStage?: string,
): string | null {
  const idx = pipeline.findIndex((s) => s.name === currentStage);
  const activeIdx = idx >= 0 ? idx : states.findIndex((s) => s === 'active' || s === 'waiting');

  if (activeIdx < 0) return null;

  const stage = pipeline[activeIdx];
  const state = states[activeIdx];

  if (state === 'waiting' && stage.kind === 'approval') {
    return `等待你在「${stage.name}」点批准或驳回`;
  }
  if (stage.gateLabel) {
    return `当前质量门：${stage.gateLabel}`;
  }
  if (stage.kind === 'agent') {
    return `${stage.role} 正在执行「${stage.name}」`;
  }

  const next = pipeline[activeIdx + 1];
  if (next?.gateLabel) return `下一步质量门：${next.gateLabel}`;
  if (next?.kind === 'approval') return `下一步：${next.name}（需你审批）`;
  return next ? `下一阶段：${next.name}` : '即将完成交付';
}
