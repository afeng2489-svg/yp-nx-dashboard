import { expect } from '@playwright/test'

export const API_BASE = process.env.API_URL || 'http://localhost:8080'

export function unwrap(body: unknown): unknown {
  if (body && typeof body === 'object' && 'ok' in (body as Record<string, unknown>)) {
    const env = body as { ok: boolean; data?: unknown; error?: string }
    if (!env.ok) throw new Error(env.error ?? 'API error')
    return env.data
  }
  return body
}

export async function api(path: string, opts: RequestInit = {}): Promise<unknown> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  })
  const body = await res.json()
  if (!res.ok) throw new Error(body?.error ?? body?.message ?? `HTTP ${res.status}`)
  return unwrap(body)
}

export async function createWorkflow(name = `e2e-wf-${Date.now()}`) {
  return api('/api/v1/workflows', {
    method: 'POST',
    body: JSON.stringify({ name, description: 'e2e test', stages: [{ name: 's1', prompt: 'hello' }] }),
  }) as Promise<{ id: string }>
}

export async function deleteWorkflow(id: string) {
  await fetch(`${API_BASE}/api/v1/workflows/${id}`, { method: 'DELETE' }).catch(() => {})
}

export async function expectOk(res: Response) {
  expect([200, 201, 204]).toContain(res.status)
}
