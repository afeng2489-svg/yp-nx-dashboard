import { describe, expect, it } from 'vitest';
import {
  inferWorkspaceSignals,
  suggestWorkflowWithContext,
} from '@/data/workspaceSignals';
import { suggestWorkflowName } from '@/data/factoryQuickStart';

describe('workspaceSignals', () => {
  it('detects empty workspace', () => {
    const s = inferWorkspaceSignals('/tmp/empty', [], undefined);
    expect(s.hasWorkspace).toBe(true);
    expect(s.isEmptyWorkspace).toBe(true);
  });

  it('detects package.json', () => {
    const s = inferWorkspaceSignals('/proj', [{ path: 'package.json', is_directory: false }]);
    expect(s.hasPackageJson).toBe(true);
    expect(s.isEmptyWorkspace).toBe(false);
  });

  it('routes stack trace to quick-fix', () => {
    const r = suggestWorkflowWithContext(
      'Error: boom\n    at foo (bar.ts:10)',
      suggestWorkflowName,
      inferWorkspaceSignals('/p', [{ path: 'package.json', is_directory: false }]),
    );
    expect(r.workflowName).toBe('quick-fix');
    expect(r.hint).toContain('错误修复');
  });

  it('routes empty workspace greenfield intent', () => {
    const r = suggestWorkflowWithContext(
      '从零做一个 todo app',
      suggestWorkflowName,
      inferWorkspaceSignals('/empty', [], undefined),
    );
    expect(r.workflowName).toBe('greenfield-mvp');
  });
});
