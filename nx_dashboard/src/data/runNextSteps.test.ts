import { describe, expect, it } from 'vitest';
import {
  dismissRunOutcome,
  isRunOutcomeDismissed,
  nextStepsForRun,
} from '@/data/runNextSteps';
import type { Execution } from '@/stores/executionStore';

function mockExecution(partial: Partial<Execution>): Execution {
  return {
    id: 'exec-1',
    workflow_id: 'wf-1',
    status: 'completed',
    trigger_source: 'factory',
    ...partial,
  };
}

describe('runNextSteps', () => {
  it('solo-dev completed suggests add feature', () => {
    const steps = nextStepsForRun(mockExecution({ status: 'completed' }), 'solo-dev');
    expect(steps?.primary.label).toBe('加第一个功能');
    expect(steps?.primary.kind).toBe('prefill');
  });

  it('writing-plans completed suggests execute plan', () => {
    const steps = nextStepsForRun(
      mockExecution({ status: 'completed', variables: { goal: 'Add auth' } }),
      'writing-plans',
    );
    expect(steps?.primary.label).toBe('开始执行计划');
    expect(steps?.primary.workflowName).toBe('solo-dev');
  });

  it('greenfield completed suggests add feature', () => {
    const steps = nextStepsForRun(mockExecution({ status: 'completed' }), 'greenfield-mvp');
    expect(steps?.primary.label).toBe('加第一个功能');
  });

  it('failed run offers retry and quick-fix switch', () => {
    const steps = nextStepsForRun(
      mockExecution({
        status: 'failed',
        current_stage: '实现',
        error: 'gate failed',
        variables: { task: 'fix login' },
      }),
      'solo-dev',
    );
    expect(steps?.primary.kind).toBe('retry');
    expect(steps?.primary.label).toContain('实现');
    expect(steps?.secondary?.label).toBe('切换快速修复');
  });

  it('quality gate failure offers three recovery actions', () => {
    const steps = nextStepsForRun(
      mockExecution({
        status: 'failed',
        variables: { task: 'fix tests' },
        stage_results: [
          {
            stage_name: '测试',
            quality_gate_result: {
              passed: false,
              retry_count: 2,
              checks: [{ cmd: 'cargo test', passed: false, exit_code: 1, stdout: '', stderr: '', duration_ms: 0 }],
            },
          },
        ],
      }),
      'solo-dev',
    );
    expect(steps?.title).toContain('质量门');
    expect(steps?.primary.label).toBe('AI 再修一轮');
    expect(steps?.secondary?.label).toBe('打开终端');
    expect(steps?.tertiary?.label).toBe('跳过此门·高级');
    expect(steps?.tertiary?.skipQualityGateForStage).toBe('测试');
  });

  it('dismiss persists in localStorage until cleared', () => {
    localStorage.clear();
    expect(isRunOutcomeDismissed('exec-1')).toBe(false);
    dismissRunOutcome('exec-1');
    expect(isRunOutcomeDismissed('exec-1')).toBe(true);
    expect(localStorage.getItem('factory-dismissed-run:exec-1')).toBe('1');
  });
});
