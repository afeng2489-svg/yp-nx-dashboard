import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Group Sessions & Processes', () => {
  let gsId: string

  test('create group session', async () => {
    const gs = await api('/api/v1/group-sessions', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-gs-${Date.now()}`, members: [] }),
    }) as { id: string }
    gsId = gs.id
    expect(gsId).toBeTruthy()
  })

  test('get group session', async () => {
    const gs = await api(`/api/v1/group-sessions/${gsId}`) as { id: string }
    expect(gs.id).toBe(gsId)
  })

  test('list group sessions', async () => {
    const list = await api('/api/v1/group-sessions') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('list processes', async () => {
    const list = await api('/api/v1/processes') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('temp cleanup', async () => {
    const res = await fetch(`${API_BASE}/api/v1/temp-cleanup`, { method: 'POST' })
    expect([200, 204]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (gsId) await fetch(`${API_BASE}/api/v1/group-sessions/${gsId}`, { method: 'DELETE' }).catch(() => {})
  })
})
