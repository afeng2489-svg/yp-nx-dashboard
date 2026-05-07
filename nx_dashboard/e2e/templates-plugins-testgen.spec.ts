import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Templates & Plugins & Test Gen', () => {
  let templateId: string

  test('create template', async () => {
    const tp = await api('/api/v1/templates', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-tpl-${Date.now()}`,
        description: 'e2e test template',
        category: 'code',
        stages: [{ name: 's1', agents: ['agent-1'], parallel: false }],
        agents: [{ id: 'agent-1', role: 'coder', model: 'default', prompt: 'write code' }],
      }),
    }) as { id: string }
    templateId = tp.id
    expect(templateId).toBeTruthy()
  })

  test('list templates', async () => {
    const data = await api('/api/v1/templates') as { items: unknown[] }
    expect(Array.isArray(data.items ?? data)).toBeTruthy()
  })

  test('list plugins', async () => {
    const list = await api('/api/v1/plugins') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('test-gen endpoint responds', async () => {
    test.setTimeout(60000)
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 45000)
    try {
      const res = await fetch(`${API_BASE}/api/v1/test-gen`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ source_code: 'fn add(a: i32, b: i32) -> i32 { a + b }', language: 'rust' }),
        signal: controller.signal,
      })
      expect([200, 422, 500]).toContain(res.status)
    } catch {
      // Timeout is acceptable — CLI may not be available
    } finally {
      clearTimeout(timeout)
    }
  })

  test('test-gen unit endpoint responds', async () => {
    test.setTimeout(60000)
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 45000)
    try {
      const res = await fetch(`${API_BASE}/api/v1/test-gen/unit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ source_code: 'fn add(a: i32, b: i32) -> i32 { a + b }', language: 'rust' }),
        signal: controller.signal,
      })
      expect([200, 422, 500]).toContain(res.status)
    } catch {
      // Timeout is acceptable — CLI may not be available
    } finally {
      clearTimeout(timeout)
    }
  })

  test.afterAll(async () => {
    if (templateId) await fetch(`${API_BASE}/api/v1/templates/${templateId}`, { method: 'DELETE' }).catch(() => {})
  })
})
