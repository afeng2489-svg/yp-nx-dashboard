import type { Workspace } from '@/stores/workspaceStore';

/** 工作区绑定的团队 ID（settings.team_id 或 API 顶层字段） */
export function workspaceTeamId(workspace: Workspace): string | undefined {
  const direct = workspace.team_id?.trim();
  if (direct) return direct;
  const fromSettings = workspace.settings?.team_id;
  return typeof fromSettings === 'string' && fromSettings.trim()
    ? fromSettings.trim()
    : undefined;
}

/** 某团队绑定的全部工作区 */
export function workspacesForTeam(workspaces: Workspace[], teamId: string): Workspace[] {
  return workspaces.filter((w) => workspaceTeamId(w) === teamId);
}

/** 按 ID 解析工作区名称（含 legacy project_id 即 workspace_id） */
export function workspaceDisplayName(
  workspaces: Workspace[],
  workspaceId: string | undefined,
): string | undefined {
  if (!workspaceId) return undefined;
  return workspaces.find((w) => w.id === workspaceId)?.name ?? workspaceId.slice(0, 8);
}
