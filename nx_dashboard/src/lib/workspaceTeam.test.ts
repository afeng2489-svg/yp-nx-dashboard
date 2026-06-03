import { describe, expect, it } from 'vitest';
import type { Workspace } from '@/stores/workspaceStore';
import { workspaceDisplayName, workspaceTeamId, workspacesForTeam } from './workspaceTeam';

const ws = (id: string, team_id?: string, settings?: Workspace['settings']): Workspace => ({
  id,
  name: `WS-${id}`,
  owner_id: 'default',
  team_id,
  settings,
  created_at: '',
  updated_at: '',
});

describe('workspaceTeam', () => {
  it('reads team_id from top-level field', () => {
    expect(workspaceTeamId(ws('a', 'team-1'))).toBe('team-1');
  });

  it('reads team_id from settings', () => {
    expect(workspaceTeamId(ws('a', undefined, { team_id: 'team-2' }))).toBe('team-2');
  });

  it('filters workspaces for team', () => {
    const list = [ws('1', 't1'), ws('2', 't2'), ws('3', 't1')];
    expect(workspacesForTeam(list, 't1').map((w) => w.id)).toEqual(['1', '3']);
  });

  it('resolves display name by workspace id', () => {
    const list = [ws('abc', 't1')];
    expect(workspaceDisplayName(list, 'abc')).toBe('WS-abc');
    expect(workspaceDisplayName(list, 'missing')).toBe('missing');
  });
});
