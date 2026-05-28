import { API_BASE_URL } from '@/api/constants';

export interface RoleExecuteResult {
  ok: boolean;
  output?: string;
  error?: string;
}

/** AF-UX-06：单角色派活 */
export async function executeRoleTask(roleId: string, task: string): Promise<RoleExecuteResult> {
  const res = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}/execute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      role_id: roleId,
      task,
      context: { source: 'factory_at_panel' },
    }),
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    return { ok: false, error: body?.error ?? body?.message ?? `HTTP ${res.status}` };
  }
  const data = body.data ?? body;
  return {
    ok: data.success !== false,
    output: data.final_output ?? data.output ?? '',
    error: data.error,
  };
}
