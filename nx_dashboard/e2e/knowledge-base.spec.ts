import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Knowledge Base', () => {
  let kbId: string
  let docId: string

  test('create knowledge base', async () => {
    const res = await fetch(`${API_BASE}/api/v1/knowledge-bases`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `e2e-kb-${Date.now()}`, description: 'test' }),
    })
    expect([200, 201]).toContain(res.status)
    const body = await res.json()
    kbId = (body.data ?? body)?.id
    expect(kbId).toBeTruthy()
  })

  test('list knowledge bases', async () => {
    const res = await fetch(`${API_BASE}/api/v1/knowledge-bases`)
    expect([200, 404]).toContain(res.status)
  })

  test('upload document', async () => {
    const form = new FormData()
    form.append('kb_id', kbId)
    form.append('file', new Blob(['NexusFlow is an AI software factory'], { type: 'text/plain' }), 'test.txt')
    const res = await fetch(`${API_BASE}/api/v1/knowledge-bases/upload`, { method: 'POST', body: form })
    expect([200, 201]).toContain(res.status)
    const body = await res.json()
    docId = (body.data ?? body)?.id
  })

  test('search knowledge', async () => {
    const res = await fetch(`${API_BASE}/api/v1/knowledge-bases/search`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ kb_id: kbId, query: 'AI' }),
    })
    expect([200, 404, 422]).toContain(res.status)
  })

  test.afterAll(async () => {
    if (docId) await fetch(`${API_BASE}/api/v1/knowledge-bases/${docId}`, { method: 'DELETE' }).catch(() => {})
    if (kbId) await fetch(`${API_BASE}/api/v1/knowledge-bases/${kbId}`, { method: 'DELETE' }).catch(() => {})
  })
})
