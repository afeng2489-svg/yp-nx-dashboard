import { describe, expect, it } from 'vitest';
import { detectStackProfile, stackFailureTerminalCommand } from '@/data/stackProfile';

describe('stackProfile', () => {
  it('detects rust', () => {
    const p = detectStackProfile([{ path: 'Cargo.toml', is_directory: false }]);
    expect(p.language).toBe('rust');
    expect(p.testCmd).toBe('cargo test');
  });

  it('detects go', () => {
    const p = detectStackProfile([{ path: 'go.mod', is_directory: false }]);
    expect(p.language).toBe('go');
  });

  it('detects python', () => {
    const p = detectStackProfile([{ path: 'pyproject.toml', is_directory: false }]);
    expect(p.language).toBe('python');
  });

  it('detects typescript', () => {
    const p = detectStackProfile([
      { path: 'package.json', is_directory: false },
      { path: 'tsconfig.json', is_directory: false },
    ]);
    expect(p.language).toBe('typescript');
  });

  it('returns terminal cmd for failures', () => {
    const p = detectStackProfile([{ path: 'Cargo.toml', is_directory: false }]);
    expect(stackFailureTerminalCommand(p)).toBe('cargo test');
  });
});
