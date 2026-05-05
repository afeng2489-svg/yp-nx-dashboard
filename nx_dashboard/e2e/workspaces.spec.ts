import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Workspaces', () => {
  let wsId: string

  test('create workspace', async () => {
    const ws = await api('/api/v1/workspaces', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-ws-${Date.now()}`, root_path: '/tmp/e2e-ws' }),
    }) as { id: string }
    wsId = ws.id
    expect(wsId).toBeTruthy()
  })

  test('list workspaces', async () => {
    const list = await api('/api/v1/workspaces') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('browse files', async () => {
    const res = await fetch(`${API_BASE}/api/v1/workspaces/${wsId}/browse`)
    expect([200, 404]).toContain(res.status)
  })

  test('git diffs', async () => {
    const res = await fetch(`${API_BASE}/api/v1/workspaces/${wsId}/diffs`)
    expect([200, 404]).toContain(res.status)
  })

  test('git status', async () => {
    const res = await fetch(`${API_BASE}/api/v1/workspaces/${wsId}/git/status`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (wsId) await fetch(`${API_BASE}/api/v1/workspaces/${wsId}`, { method: 'DELETE' }).catch(() => {})
  })
})
