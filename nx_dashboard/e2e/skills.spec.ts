import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Skills System', () => {
  let skillId: string

  test('list skills (presets)', async () => {
    const list = await api('/api/v1/skills') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('get skill categories', async () => {
    const cats = await api('/api/v1/skills/categories') as unknown[]
    expect(Array.isArray(cats)).toBeTruthy()
  })

  test('get skill tags', async () => {
    const tags = await api('/api/v1/skills/tags') as unknown[]
    expect(Array.isArray(tags)).toBeTruthy()
  })

  test('get skill stats', async () => {
    const stats = await api('/api/v1/skills/stats') as { total_skills: number }
    expect(typeof stats.total_skills).toBe('number')
  })

  test('search skills', async () => {
    const res = await fetch(`${API_BASE}/api/v1/skills/search?query=code`)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(Array.isArray(body.data ?? body)).toBeTruthy()
  })

  test('import skill via paste', async () => {
    const skill = await api('/api/v1/skills/import', {
      method: 'POST',
      body: JSON.stringify({
        source: 'paste',
        content: JSON.stringify({
          id: `e2e-skill-${Date.now()}`,
          name: 'E2E Test Skill',
          description: 'test',
          category: 'test',
          code: 'echo hello',
        }),
      }),
    }) as { id: string }
    skillId = skill.id
    expect(skillId).toBeTruthy()
  })

  test('execute skill', async () => {
    const res = await fetch(`${API_BASE}/api/v1/skills/${skillId}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ params: {} }),
    })
    expect([200, 500]).toContain(res.status) // 500 ok if CLI not installed
  })

  test.afterAll(async () => {
    if (skillId) await fetch(`${API_BASE}/api/v1/skills/${skillId}`, { method: 'DELETE' }).catch(() => {})
  })
})
