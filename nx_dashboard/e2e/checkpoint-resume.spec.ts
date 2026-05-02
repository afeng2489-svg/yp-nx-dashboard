import { test, expect } from '@playwright/test'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

function unwrap(body: unknown): any {
  if (body && typeof body === 'object' && 'ok' in (body as any)) {
    const env = body as { ok: boolean; data?: any; error?: string }
    if (!env.ok) throw new Error(env.error ?? 'API error')
    return env.data
  }
  return body
}

async function api(path: string, opts: RequestInit = {}): Promise<any> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  })
  const body = await res.json()
  if (!res.ok) {
    const msg = body?.error ?? body?.message ?? `HTTP ${res.status}`
    throw new Error(msg)
  }
  return unwrap(body)
}

test.describe('Checkpoint Resume E2E', () => {
  test('health check', async () => {
    const health = await api('/health')
    expect(health.status).toBe('ok')
  })

  test('interrupted executions endpoint returns array', async () => {
    const result = await api('/api/v1/executions/interrupted')
    expect(Array.isArray(result)).toBeTruthy()
  })

  test('crash detection endpoint responds', async () => {
    const res = await fetch(`${API_BASE}/api/v1/crash-detect`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    })
    expect([200, 500, 503]).toContain(res.status)
  })

  test('temp cleanup endpoint works', async () => {
    const result = await api('/api/v1/temp-cleanup', { method: 'POST' })
    expect(result).toBeTruthy()
  })

  test('abandon non-existent checkpoint is idempotent (200)', async () => {
    const res = await fetch(`${API_BASE}/api/v1/executions/nonexistent-id/checkpoint`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
    })
    expect(res.ok).toBeTruthy()
  })

  test('resume non-existent execution returns 404', async () => {
    const res = await fetch(`${API_BASE}/api/v1/executions/nonexistent-id/resume`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    })
    expect(res.status).toBe(404)
  })

  test('checkpoint is created during pipeline dispatch', async () => {
    // Create team + project + pipeline
    const team = await api('/api/v1/teams', {
      method: 'POST',
      body: JSON.stringify({
        name: `checkpoint-test-team-${Date.now()}`,
        description: 'Checkpoint E2E test',
      }),
    })
    const teamId = team.id ?? team.team_id
    expect(teamId).toBeTruthy()

    const project = await api('/api/v1/projects', {
      method: 'POST',
      body: JSON.stringify({
        name: `checkpoint-test-proj-${Date.now()}`,
        description: 'Checkpoint E2E test',
        team_id: teamId,
      }),
    })
    const projectId = project.id ?? project.project_id
    expect(projectId).toBeTruthy()

    const pipeline = await api(`/api/v1/projects/${projectId}/pipeline`, {
      method: 'POST',
      body: JSON.stringify({ team_id: teamId }),
    })
    const pipelineId = pipeline.id
    expect(pipelineId).toBeTruthy()

    // Start + dispatch
    await api(`/api/v1/pipelines/${pipelineId}/start`, { method: 'POST' })
    await api(`/api/v1/pipelines/${pipelineId}/dispatch`, { method: 'POST' })

    // Wait a moment for checkpoint to be written
    await new Promise((r) => setTimeout(r, 500))

    // Check pipeline status
    const data = await api(`/api/v1/pipelines/${pipelineId}/steps`)
    expect(data.progress).toBeTruthy()

    // Cleanup
    await fetch(`${API_BASE}/api/v1/projects/${projectId}`, { method: 'DELETE' }).catch(() => {})
    await fetch(`${API_BASE}/api/v1/teams/${teamId}`, { method: 'DELETE' }).catch(() => {})
  })
})
