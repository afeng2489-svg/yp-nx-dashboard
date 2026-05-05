import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('AI Config', () => {
  test('list CLIs', async () => {
    const data = await api('/api/v1/ai/clis') as { clis: unknown[] }
    expect(Array.isArray(data.clis ?? data)).toBeTruthy()
  })

  test('list models', async () => {
    const models = await api('/api/v1/ai/models') as unknown[]
    expect(Array.isArray(models)).toBeTruthy()
  })

  test('get selected model', async () => {
    const res = await fetch(`${API_BASE}/api/v1/ai/selected`)
    expect([200, 404]).toContain(res.status)
  })

  test('list providers v1', async () => {
    const data = await api('/api/v1/ai/providers') as { providers: unknown[] }
    expect(Array.isArray(data.providers ?? data)).toBeTruthy()
  })

  test('list providers v2', async () => {
    const data = await api('/api/v1/ai/v2/providers') as { providers: unknown[] }
    expect(Array.isArray(data.providers ?? data)).toBeTruthy()
  })

  test('create and delete provider v2', async () => {
    const pv = await api('/api/v1/ai/v2/providers', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-pv-${Date.now()}`,
        provider_key: 'openai',
        api_key: 'sk-test',
        base_url: 'https://api.openai.com',
      }),
    }) as { id: string }
    expect(pv.id).toBeTruthy()
    await fetch(`${API_BASE}/api/v1/ai/v2/providers/${pv.id}`, { method: 'DELETE' })
  })

  test('get api keys', async () => {
    const res = await fetch(`${API_BASE}/api/v1/ai/api-keys`)
    expect([200, 404]).toContain(res.status)
  })

  test('refresh models', async () => {
    const res = await fetch(`${API_BASE}/api/v1/ai/models/refresh`, { method: 'POST' })
    expect([200, 202]).toContain(res.status)
  })
})
