/** API path prefix for workspace-scoped team evolution / pipeline endpoints */
export function workspaceScopePath(workspaceId: string, suffix: string): string {
  const base = `/api/v1/workspaces/${encodeURIComponent(workspaceId)}`;
  return suffix.startsWith('/') ? `${base}${suffix}` : `${base}/${suffix}`;
}
