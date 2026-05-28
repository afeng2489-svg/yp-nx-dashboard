import { API_BASE_URL } from '@/api/constants';

export interface CostSummary {
  total_tokens: number;
  total_cost_usd: number;
  total_executions: number;
}

export interface DailyCost {
  date: string;
  tokens: number;
  cost_usd: number;
}

function unwrap<T>(data: unknown): T {
  if (data && typeof data === 'object' && 'data' in data) {
    return (data as { data: T }).data;
  }
  return data as T;
}

export async function fetchCostSummary(): Promise<CostSummary> {
  const res = await fetch(`${API_BASE_URL}/api/v1/costs/summary`);
  if (!res.ok) throw new Error('cost summary failed');
  return unwrap<CostSummary>(await res.json());
}

export async function fetchCostByDay(days = 7): Promise<DailyCost[]> {
  const res = await fetch(`${API_BASE_URL}/api/v1/costs/by-day?days=${days}`);
  if (!res.ok) throw new Error('cost by-day failed');
  const raw = unwrap<DailyCost[]>(await res.json());
  return Array.isArray(raw) ? raw : [];
}

/** 今日 + 近 7 日 USD 合计 */
export async function fetchSpendSnapshot(): Promise<{ todayUsd: number; weekUsd: number }> {
  const rows = await fetchCostByDay(7);
  const today = new Date().toISOString().slice(0, 10);
  let todayUsd = 0;
  let weekUsd = 0;
  for (const row of rows) {
    weekUsd += row.cost_usd ?? 0;
    if (row.date === today) todayUsd = row.cost_usd ?? 0;
  }
  return { todayUsd, weekUsd };
}
