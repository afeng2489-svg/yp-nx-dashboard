import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Teams & Roles', () => {
  let teamId: string
  let roleId: string

  test('create team', async () => {
    const tm = await api('/api/v1/teams', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-team-${Date.now()}`, description: 'test' }),
    }) as { id: string }
    teamId = tm.id
    expect(teamId).toBeTruthy()
  })

  test('list teams', async () => {
    const list = await api('/api/v1/teams') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('get team roles', async () => {
    const roles = await api(`/api/v1/teams/${teamId}/roles`) as unknown[]
    expect(Array.isArray(roles)).toBeTruthy()
  })

  test('create role', async () => {
    const rl = await api('/api/v1/roles', {
      method: 'POST',
      body: JSON.stringify({ name: 'developer', description: 'code', team_id: teamId }),
    }) as { id: string }
    roleId = rl.id
    expect(roleId).toBeTruthy()
  })

  test('get role skills', async () => {
    const res = await fetch(`${API_BASE}/api/v1/roles/${roleId}/skills`)
    expect([200, 404]).toContain(res.status)
  })

  test('get role team', async () => {
    const res = await fetch(`${API_BASE}/api/v1/roles/${roleId}/team`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (roleId) await fetch(`${API_BASE}/api/v1/roles/${roleId}`, { method: 'DELETE' }).catch(() => {})
    if (teamId) await fetch(`${API_BASE}/api/v1/teams/${teamId}`, { method: 'DELETE' }).catch(() => {})
  })
})
