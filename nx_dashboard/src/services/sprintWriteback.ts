import { API_BASE_URL } from '@/api/constants';

let sprintIdCache: Set<string> | null = null;

async function sprintExists(sprintId: string): Promise<boolean> {
  try {
    if (!sprintIdCache) {
      const res = await fetch(`${API_BASE_URL}/api/v1/sprints`);
      if (!res.ok) return false;
      const cards = (await res.json()) as { id: string }[];
      sprintIdCache = new Set(cards.map((c) => c.id));
    }
    return sprintIdCache.has(sprintId);
  } catch {
    return false;
  }
}

/** Sprint 卡片 → Run 启动时标记进行中 */
export async function markSprintInProgress(sprintId: string, detail?: string) {
  if (!(await sprintExists(sprintId))) return;
  try {
    const statusRes = await fetch(`${API_BASE_URL}/api/v1/sprints/${sprintId}/status`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'in_progress' }),
    });
    if (!statusRes.ok) return;

    await fetch(`${API_BASE_URL}/api/v1/sprints/${sprintId}/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        event_type: 'started',
        detail: detail ?? '从工厂台启动 Run',
      }),
    });
  } catch {
    /* best-effort */
  }
}

/** Run 完成/失败后回写 Sprint 看板 */
export async function writebackSprintOnRunComplete(
  sprintId: string,
  runStatus: 'completed' | 'failed' | 'cancelled',
) {
  if (!(await sprintExists(sprintId))) return;
  const cardStatus = runStatus === 'completed' ? 'completed' : 'blocked';
  const eventType = runStatus === 'completed' ? 'completed' : 'blocked';
  try {
    const statusRes = await fetch(`${API_BASE_URL}/api/v1/sprints/${sprintId}/status`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: cardStatus }),
    });
    if (!statusRes.ok) return;

    await fetch(`${API_BASE_URL}/api/v1/sprints/${sprintId}/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        event_type: eventType,
        detail: `Run ${runStatus}`,
      }),
    });
  } catch {
    /* best-effort */
  }
}
