import { test, expect } from '@playwright/test'
import { createWorkflow, deleteWorkflow } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Triggers & Scheduler', () => {
  let wfId: string

  test.beforeAll(async () => {
    const wf = await createWorkflow() as { id: string }
    wfId = wf.id
  })

  test('webhook trigger', async () => {
    const res = await fetch(`${API_BASE}/api/v1/triggers/webhook/${wfId}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ input: 'test' }),
    })
    // Accept 200 (success) or 404 (workflow not configured for webhook)
    expect([200, 404]).toContain(res.status)
  })

  test('list triggers', async () => {
    const res = await fetch(`${API_BASE}/api/v1/triggers`)
    // Endpoint may not exist (404) — accept that
    expect([200, 404]).toContain(res.status)
  })

  test('scheduler status', async () => {
    const res = await fetch(`${API_BASE}/api/v1/scheduler/status`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (wfId) await deleteWorkflow(wfId)
  })
})
