import { test, expect } from '@playwright/test'
import { api } from './helpers'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe.configure({ mode: 'serial' })

test.describe('Tasks & Projects & Issues', () => {
  let taskId: string
  let projectId: string
  let issueId: string

  test('create task', async () => {
    const tk = await api('/api/v1/tasks', {
      method: 'POST',
      body: JSON.stringify({ title: `e2e-task-${Date.now()}`, status: 'pending' }),
    }) as { id: string }
    taskId = tk.id
    expect(taskId).toBeTruthy()
  })

  test('update task status', async () => {
    const tk = await api(`/api/v1/tasks/${taskId}`, {
      method: 'PUT',
      body: JSON.stringify({ status: 'in_progress' }),
    }) as { status: string }
    expect(tk.status).toBe('in_progress')
  })

  test('task stats', async () => {
    const stats = await api('/api/v1/tasks/stats')
    expect(stats).toBeTruthy()
  })

  test('create project', async () => {
    const pj = await api('/api/v1/projects', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-proj-${Date.now()}`, description: 'test' }),
    }) as { id: string }
    projectId = pj.id
    expect(projectId).toBeTruthy()
  })

  test('list projects', async () => {
    const list = await api('/api/v1/projects') as unknown[]
    expect(Array.isArray(list)).toBeTruthy()
  })

  test('create issue', async () => {
    const is = await api('/api/v1/issues', {
      method: 'POST',
      body: JSON.stringify({ title: `e2e-issue-${Date.now()}`, severity: 'medium' }),
    }) as { id: string }
    issueId = is.id
    expect(issueId).toBeTruthy()
  })

  test('resolve issue', async () => {
    const is = await api(`/api/v1/issues/${issueId}`, {
      method: 'PUT',
      body: JSON.stringify({ status: 'resolved' }),
    }) as { status: string }
    expect(is.status).toBe('resolved')
  })

  test.afterAll(async () => {
    for (const [path, id] of [['tasks', taskId], ['projects', projectId], ['issues', issueId]]) {
      if (id) await fetch(`${API_BASE}/api/v1/${path}/${id}`, { method: 'DELETE' }).catch(() => {})
    }
  })
})
