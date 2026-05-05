import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Costs & Token Monitoring', () => {
  test('cost summary', async () => {
    const res = await fetch(`${API_BASE}/api/v1/costs/summary`)
    expect([200, 404]).toContain(res.status)
    if (res.status === 200) {
      const body = await res.json()
      expect(body).toBeTruthy()
    }
  })

  test('cost by day', async () => {
    const res = await fetch(`${API_BASE}/api/v1/costs/by-day?days=7`)
    expect([200, 404]).toContain(res.status)
  })
})

test.describe('Search', () => {
  test('search modes', async () => {
    const modes = await api('/api/v1/search/modes') as unknown[]
    expect(Array.isArray(modes)).toBeTruthy()
  })

  test('search query', async () => {
    const res = await fetch(`${API_BASE}/api/v1/search?q=workflow`)
    expect([200, 404]).toContain(res.status)
  })

  test('reindex', async () => {
    const res = await fetch(`${API_BASE}/api/v1/search/index`, { method: 'POST' })
    expect([200, 202]).toContain(res.status)
  })
})

test.describe('Wisdom', () => {
  let wisdomId: string

  test('create wisdom', async () => {
    const wd = await api('/api/v1/wisdom', {
      method: 'POST',
      body: JSON.stringify({ title: `e2e-wisdom-${Date.now()}`, content: 'test insight', category: 'engineering' }),
    }) as { id: string }
    wisdomId = wd.id
    expect(wisdomId).toBeTruthy()
  })

  test('list categories', async () => {
    const cats = await api('/api/v1/wisdom/categories') as unknown[]
    expect(Array.isArray(cats)).toBeTruthy()
  })

  test('search wisdom', async () => {
    const res = await fetch(`${API_BASE}/api/v1/wisdom/search?q=insight`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (wisdomId) await fetch(`${API_BASE}/api/v1/wisdom/${wisdomId}`, { method: 'DELETE' }).catch(() => {})
  })
})
