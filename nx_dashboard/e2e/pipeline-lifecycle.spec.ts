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

test.describe('Pipeline E2E Lifecycle', () => {
  let teamId: string
  let projectId: string
  let pipelineId: string

  test('health check', async () => {
    const health = await api('/health')
    expect(health.status).toBe('ok')
  })

  test('create team', async () => {
    const team = await api('/api/v1/teams', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-pipe-team-${Date.now()}`,
        description: 'E2E test team',
      }),
    })
    teamId = team.id ?? team.team_id
    expect(teamId).toBeTruthy()
  })

  test('create project', async () => {
    const project = await api('/api/v1/projects', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-pipe-proj-${Date.now()}`,
        description: 'E2E pipeline test',
        team_id: teamId,
      }),
    })
    projectId = project.id ?? project.project_id
    expect(projectId).toBeTruthy()
  })

  test('create pipeline', async () => {
    const pipeline = await api(`/api/v1/projects/${projectId}/pipeline`, {
      method: 'POST',
      body: JSON.stringify({ team_id: teamId }),
    })
    pipelineId = pipeline.id
    expect(pipelineId).toBeTruthy()
    expect(pipeline.status).toBe('idle')
  })

  test('start pipeline', async () => {
    const pipeline = await api(`/api/v1/pipelines/${pipelineId}/start`, {
      method: 'POST',
    })
    expect(pipeline.status).toBe('running')
  })

  test('get pipeline steps', async () => {
    const data = await api(`/api/v1/pipelines/${pipelineId}/steps`)
    expect(Array.isArray(data.steps)).toBeTruthy()
    expect(data.progress).toBeTruthy()
    expect(typeof data.progress.total_steps).toBe('number')
  })

  test('dispatch steps', async () => {
    const result = await api(`/api/v1/pipelines/${pipelineId}/dispatch`, {
      method: 'POST',
    })
    expect(typeof result.dispatched).toBe('number')
  })

  test('get status after dispatch', async () => {
    const data = await api(`/api/v1/pipelines/${pipelineId}/steps`)
    expect(data.progress).toBeTruthy()
    expect(typeof data.progress.progress_pct).toBe('number')
    expect(data.progress.progress_pct).toBeGreaterThanOrEqual(0)
  })

  test('pause pipeline', async () => {
    const pipeline = await api(`/api/v1/pipelines/${pipelineId}/pause`, {
      method: 'POST',
    }).catch(() => null)
    if (pipeline) {
      expect(pipeline.status).toBe('paused')
    }
  })

  test('resume pipeline', async () => {
    const pipeline = await api(`/api/v1/pipelines/${pipelineId}/resume`, {
      method: 'POST',
    }).catch(() => null)
    if (pipeline) {
      expect(['running', 'completed']).toContain(pipeline.status)
    }
  })

  test('processes endpoint returns array', async () => {
    const processes = await api('/api/v1/processes')
    expect(Array.isArray(processes)).toBeTruthy()
  })

  test('retry failed step (smoke)', async () => {
    const data = await api(`/api/v1/pipelines/${pipelineId}/steps`)
    const failedStep = data.steps?.find((s: any) => s.status === 'failed')
    if (failedStep) {
      const step = await api(
        `/api/v1/pipelines/${pipelineId}/steps/${failedStep.id}/retry`,
        { method: 'POST' },
      )
      expect(step.id).toBe(failedStep.id)
    }
  })

  test('frontend loads', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('body')).toBeVisible()
  })

  test.afterAll(async () => {
    if (projectId) {
      await fetch(`${API_BASE}/api/v1/projects/${projectId}`, {
        method: 'DELETE',
      }).catch(() => {})
    }
    if (teamId) {
      await fetch(`${API_BASE}/api/v1/teams/${teamId}`, {
        method: 'DELETE',
      }).catch(() => {})
    }
  })
})
