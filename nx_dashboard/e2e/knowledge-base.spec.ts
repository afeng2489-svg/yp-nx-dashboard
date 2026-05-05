import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Knowledge Base', () => {
  let docId: string

  test('list knowledge docs', async () => {
    const res = await fetch(`${API_BASE}/api/v1/knowledge`)
    expect([200, 404]).toContain(res.status)
  })

  test('upload document', async () => {
    const form = new FormData()
    form.append('file', new Blob(['NexusFlow is an AI software factory'], { type: 'text/plain' }), 'test.txt')
    const res = await fetch(`${API_BASE}/api/v1/knowledge`, { method: 'POST', body: form })
    expect([200, 201]).toContain(res.status)
    const body = await res.json()
    docId = (body.data ?? body)?.id
  })

  test('search knowledge', async () => {
    const res = await fetch(`${API_BASE}/api/v1/knowledge/search?q=AI`)
    expect([200, 404]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (docId) await fetch(`${API_BASE}/api/v1/knowledge/${docId}`, { method: 'DELETE' }).catch(() => {})
  })
})
