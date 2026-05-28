import { describe, expect, it } from 'vitest';
import { buildLaunchPreview, estimateCliCostUsd } from '@/data/launchPreview';
import { isVisibleInMoreDrawer, workflowTier } from '@/data/workflowTiers';
import { isTextOnlyWorkflow } from '@/data/textOnlyWorkflows';

describe('launchPreview', () => {
  it('builds preview for solo-dev', () => {
    const p = buildLaunchPreview('solo-dev');
    expect(p.stageCount).toBe(6);
    expect(p.estimatedCostUsd).toBeGreaterThan(0);
  });
});

describe('workflowTiers', () => {
  it('tiers dev-workflow as tier 1', () => {
    expect(workflowTier('dev-workflow')).toBe(1);
  });

  it('hides tier 3 from more drawer', () => {
    expect(isVisibleInMoreDrawer('ui-ux-design')).toBe(false);
    expect(isVisibleInMoreDrawer('writing-plans')).toBe(true);
  });
});

describe('textOnlyWorkflows', () => {
  it('writing-plans is text only', () => {
    expect(isTextOnlyWorkflow('writing-plans')).toBe(true);
    expect(isTextOnlyWorkflow('solo-dev')).toBe(false);
  });
});
