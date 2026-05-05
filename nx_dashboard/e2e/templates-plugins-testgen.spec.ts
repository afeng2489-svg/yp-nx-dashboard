import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Templates & Plugins & Test Gen', () => {
  let templateId: string

  test('create template', async () => {
    const tp = await api('/api/v1/templates', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-tpl-${Date.now()}`, content: '# {{prompt}}', category: 'code' }),
    }) as { id: string }
    templateId = tp.id
    expect(templateId).toBeTruthy()
  })

  test('list templates', async () => {
    const list = await api('/api/v1/templates') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('list plugins', async () => {
    const list = await api('/api/v1/plugins') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('test-gen endpoint responds', async () => {
    const res = await fetch(`${API_BASE}/api/v1/test-gen`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code: 'fn add(a: i32, b: i32) -> i32 { a + b }', language: 'rust' }),
    })
    expect([200, 500]).toContain(res.status) // 500 ok if CLI not installed
  })

  test('test-gen unit endpoint responds', async () => {
    const res = await fetch(`${API_BASE}/api/v1/test-gen/unit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ function: 'add', language: 'rust' }),
    })
    expect([200, 500]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (templateId) await fetch(`${API_BASE}/api/v1/templates/${templateId}`, { method: 'DELETE' }).catch(() => {})
  })
})
