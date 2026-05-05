import { test, expect } from '@playwright/test'
import { api, createWorkflow, deleteWorkflow } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Triggers & Scheduler', () => {
  let wfId: string
  let triggerId: string

  test.beforeAll(async () => {
    const wf = await createWorkflow() as { id: string }
    wfId = wf.id
  })

  test('create trigger', async () => {
    const tr = await api('/api/v1/triggers', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-trigger-${Date.now()}`, type: 'manual', workflow_id: wfId }),
    }) as { id: string }
    triggerId = tr.id
    expect(triggerId).toBeTruthy()
  })

  test('list triggers', async () => {
    const list = await api('/api/v1/triggers') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('scheduler status', async () => {
    const res = await fetch(`${API_BASE}/api/v1/scheduler/status`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (triggerId) await fetch(`${API_BASE}/api/v1/triggers/${triggerId}`, { method: 'DELETE' }).catch(() => {})
    if (wfId) await deleteWorkflow(wfId)
  })
})
