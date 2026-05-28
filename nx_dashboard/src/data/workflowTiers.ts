/** AF-15：内部 Tier 注册 — 不对用户暴露 workflow id */

export type WorkflowTier = 1 | 2 | 3;

const TIER_MAP: Record<string, WorkflowTier> = {
  'greenfield-mvp': 1,
  'solo-dev': 1,
  'quick-fix': 1,
  'dev-workflow': 1,
  'writing-plans': 2,
  'review-cycle': 2,
  investigate: 2,
  'ui-ux-design': 3,
  'page-generate': 3,
  brainstorm: 3,
};

/** 首屏「更多方式」折叠区允许的最大 Tier */
export const MORE_DRAWER_MAX_TIER: WorkflowTier = 2;

export function workflowTier(name: string): WorkflowTier {
  return TIER_MAP[name] ?? 3;
}

export function isVisibleInMoreDrawer(workflowName: string): boolean {
  return workflowTier(workflowName) <= MORE_DRAWER_MAX_TIER;
}
