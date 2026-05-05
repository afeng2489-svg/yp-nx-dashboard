import { test, expect } from '@playwright/test'
import { api, createWorkflow, deleteWorkflow } from './helpers'

test.describe.configure({ mode: 'serial' })

test.describe('Execution Lifecycle', () => {
  let wfId: string
  let exId: string

  test.beforeAll(async () => {
    const wf = await createWorkflow() as { id: string }
    wfId = wf.id
  })

  test('create execution', async () => {
    const ex = await api('/api/v1/executions', {
      method: 'POST',
      body: JSON.stringify({ workflow_id: wfId, input: {} }),
    }) as { id: string; status: string }
    exId = ex.id
    expect(exId).toBeTruthy()
    expect(['pending', 'running']).toContain(ex.status)
  })

  test('get execution', async () => {
    const ex = await api(`/api/v1/executions/${exId}`) as { id: string }
    expect(ex.id).toBe(exId)
  })

  test('list executions', async () => {
    const list = await api('/api/v1/executions') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('execution git info', async () => {
    const res = await fetch(`${process.env.API_URL || 'http://localhost:8080'}/api/v1/executions/${exId}/git`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => { if (wfId) await deleteWorkflow(wfId) })
})
