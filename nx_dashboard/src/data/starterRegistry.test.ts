import { describe, expect, it } from 'vitest';
import { STARTER_REGISTRY, GREENFIELD_REGISTRY_PRESETS, starterByStack } from '@/data/starterRegistry';

describe('starterRegistry', () => {
  it('has greenfield entries', () => {
    expect(GREENFIELD_REGISTRY_PRESETS.length).toBeGreaterThanOrEqual(5);
  });

  it('finds go-api starter', () => {
    expect(starterByStack('go-api')?.workflow).toBe('greenfield-mvp');
  });

  it('includes python-fastapi and rust-cli', () => {
    expect(STARTER_REGISTRY.some((s) => s.id === 'python-fastapi')).toBe(true);
    expect(STARTER_REGISTRY.some((s) => s.id === 'rust-cli')).toBe(true);
  });
});
