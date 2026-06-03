import { describe, expect, it } from 'vitest';
import {
  inferWorkspaceSignals,
  suggestWorkflowWithContext,
  filterFactoryQuickCards,
} from '@/data/workspaceSignals';
import { suggestWorkflowName } from '@/data/factoryQuickStart';
import { FACTORY_QUICK_LINES } from '@/data/factoryQuickStart';

describe('workspaceSignals', () => {
  it('detects empty workspace', () => {
    const s = inferWorkspaceSignals('/tmp/empty', [], undefined);
    expect(s.hasWorkspace).toBe(true);
    expect(s.isEmptyWorkspace).toBe(true);
    expect(s.hasExistingCode).toBe(false);
  });

  it('detects rust project', () => {
    const s = inferWorkspaceSignals('/proj', [{ path: 'Cargo.toml', is_directory: false }]);
    expect(s.hasExistingCode).toBe(true);
    expect(s.stack.language).toBe('rust');
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

  it('routes existing rust repo to solo-dev', () => {
    const r = suggestWorkflowWithContext(
      '加一个子命令',
      suggestWorkflowName,
      inferWorkspaceSignals('/p', [{ path: 'Cargo.toml', is_directory: false }]),
    );
    expect(r.workflowName).toBe('solo-dev');
    expect(r.hint).toContain('rust');
  });

  it('routes empty workspace greenfield intent', () => {
    const r = suggestWorkflowWithContext(
      '从零做一个 todo app',
      suggestWorkflowName,
      inferWorkspaceSignals('/empty', [], undefined),
    );
    expect(r.workflowName).toBe('greenfield-mvp');
  });

  it('filters ui-design card for rust workspace', () => {
    const signals = inferWorkspaceSignals('/p', [{ path: 'Cargo.toml', is_directory: false }]);
    const cards = filterFactoryQuickCards(
      FACTORY_QUICK_LINES.map((item) => ({ ...item })),
      signals,
    );
    expect(cards.some((c) => c.id === 'ui-design')).toBe(false);
    expect(cards.some((c) => c.id === 'solo-dev')).toBe(true);
  });
});
