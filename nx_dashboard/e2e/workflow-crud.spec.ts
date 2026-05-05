import { test, expect } from '@playwright/test'
import { api, createWorkflow, deleteWorkflow } from './helpers'

test.describe.configure({ mode: 'serial' })

test.describe('Workflow CRUD', () => {
  let wfId: string

  test('create workflow', async () => {
    const wf = await createWorkflow() as { id: string }
    wfId = wf.id
    expect(wfId).toBeTruthy()
  })

  test('get workflow', async () => {
    const wf = await api(`/api/v1/workflows/${wfId}`) as { id: string; name: string }
    expect(wf.id).toBe(wfId)
  })

  test('update workflow', async () => {
    const wf = await api(`/api/v1/workflows/${wfId}`, {
      method: 'PUT',
      body: JSON.stringify({ name: 'e2e-updated' }),
    }) as { name: string }
    expect(wf.name).toBe('e2e-updated')
  })

  test('list workflows', async () => {
    const list = await api('/api/v1/workflows') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
    expect(list.length).toBeGreaterThan(0)
  })

  test.afterAll(async () => { if (wfId) await deleteWorkflow(wfId) })
})
