import { describe, expect, it } from 'vitest';
import {
  SOLO_DEV_PIPELINE,
  QUICK_FIX_PIPELINE,
  inferCurrentStageName,
  nextGateHint,
  pipelineForWorkflow,
  pipelineLabelForWorkflow,
  formatPipelineStageSummary,
  resolveStageStates,
} from '@/data/workflowPipelines';

describe('workflowPipelines', () => {
  it('solo-dev has four stages including approval', () => {
    expect(SOLO_DEV_PIPELINE).toHaveLength(4);
    expect(SOLO_DEV_PIPELINE.map((s) => s.name)).toEqual([
      '开发',
      '交付审批',
      '审查',
      '交付摘要',
    ]);
    expect(SOLO_DEV_PIPELINE.some((s) => s.name === '交付审批' && s.kind === 'approval')).toBe(true);
  });

  it('quick-fix has five stages including approval', () => {
    expect(QUICK_FIX_PIPELINE).toHaveLength(5);
    expect(QUICK_FIX_PIPELINE.map((s) => s.name)).toEqual([
      '分析',
      '修复',
      '验证',
      '交付审批',
      '交付摘要',
    ]);
  });

  it('pipelineForWorkflow resolves registry keys and falls back to solo-dev', () => {
    expect(pipelineForWorkflow('quick-fix')).toBe(QUICK_FIX_PIPELINE);
    expect(pipelineForWorkflow('unknown-wf')).toBe(SOLO_DEV_PIPELINE);
    expect(pipelineForWorkflow()).toBe(SOLO_DEV_PIPELINE);
  });

  it('pipelineLabelForWorkflow uses Chinese labels not raw ids', () => {
    expect(pipelineLabelForWorkflow('solo-dev')).toBe('一人全栈');
    expect(pipelineLabelForWorkflow('quick-fix')).toBe('快速修复');
    expect(pipelineLabelForWorkflow('solo-dev')).not.toBe('solo-dev');
  });

  it('formatPipelineStageSummary joins stage names', () => {
    expect(formatPipelineStageSummary('quick-fix')).toBe(
      '分析 → 修复 → 验证 → 交付审批 → 交付摘要',
    );
  });

  it('resolveStageStates marks completed and active', () => {
    const states = resolveStageStates(
      SOLO_DEV_PIPELINE,
      ['开发'],
      '交付审批',
      'running',
    );
    expect(states[0]).toBe('done');
    expect(states[1]).toBe('active');
    expect(states[2]).toBe('pending');
  });

  it('resolveStageStates shows waiting on paused approval', () => {
    const states = resolveStageStates(
      SOLO_DEV_PIPELINE,
      ['开发'],
      '交付审批',
      'paused',
    );
    expect(states[1]).toBe('waiting');
  });

  it('inferCurrentStageName prefers pending pause', () => {
    expect(
      inferCurrentStageName('开发', '交付审批', [{ stage_name: '开发' }]),
    ).toBe('交付审批');
  });

  it('nextGateHint for approval pause', () => {
    const states = resolveStageStates(
      SOLO_DEV_PIPELINE,
      ['开发'],
      '交付审批',
      'paused',
    );
    const hint = nextGateHint(SOLO_DEV_PIPELINE, states, '交付审批');
    expect(hint).toContain('交付审批');
    expect(hint).toContain('批准');
  });
});
