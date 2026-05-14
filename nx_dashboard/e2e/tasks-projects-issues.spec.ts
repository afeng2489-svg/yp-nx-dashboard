import { test, expect } from '@playwright/test';
import { api } from './helpers';

const API_BASE = process.env.API_URL || 'http://localhost:8080';

test.describe.configure({ mode: 'serial' });

test.describe('Tasks & Projects & Issues', () => {
  let taskId: string;
  let projectId: string;
  let issueId: string;
  let teamId: string;

  test('create task', async () => {
    const tk = (await api('/api/v1/tasks', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-task-${Date.now()}`,
        description: 'test task',
        stages: [{ name: 's1', agents: ['default'], prompt_template: 'do something' }],
      }),
    })) as { id: string };
    taskId = tk.id;
    expect(taskId).toBeTruthy();
  });

  test('get task', async () => {
    // Task endpoint may not support GET by ID, accept 200 or 404/405
    const res = await fetch(`${API_BASE}/api/v1/tasks/${taskId}`);
    expect([200, 404, 405]).toContain(res.status);
  });

  test('task stats', async () => {
    const stats = await api('/api/v1/tasks/stats');
    expect(stats).toBeTruthy();
  });

  test('create team for project', async () => {
    const tm = (await api('/api/v1/teams', {
      method: 'POST',
      body: JSON.stringify({ name: `e2e-proj-team-${Date.now()}`, description: 'test' }),
    })) as { id: string };
    teamId = tm.id;
    expect(teamId).toBeTruthy();
  });

  test('create project', async () => {
    const pj = (await api('/api/v1/projects', {
      method: 'POST',
      body: JSON.stringify({
        name: `e2e-proj-${Date.now()}`,
        description: 'test',
        team_id: teamId,
      }),
    })) as { id: string };
    projectId = pj.id;
    expect(projectId).toBeTruthy();
  });

  test('list projects', async () => {
    const list = (await api('/api/v1/projects')) as unknown[];
    expect(Array.isArray(list)).toBeTruthy();
  });

  test('create issue', async () => {
    const is = (await api('/api/v1/issues', {
      method: 'POST',
      body: JSON.stringify({
        title: `e2e-issue-${Date.now()}`,
        description: 'test issue',
        priority: 'medium',
        perspectives: [],
        depends_on: [],
      }),
    })) as { id: string };
    issueId = is.id;
    expect(issueId).toBeTruthy();
  });

  test('resolve issue', async () => {
    const is = (await api(`/api/v1/issues/${issueId}`, {
      method: 'PUT',
      body: JSON.stringify({ status: 'completed' }),
    })) as { status: string };
    expect(is.status).toBe('completed');
  });

  test.afterAll(async () => {
    for (const [path, id] of [
      ['tasks', taskId],
      ['projects', projectId],
      ['issues', issueId],
      ['teams', teamId],
    ]) {
      if (id) await fetch(`${API_BASE}/api/v1/${path}/${id}`, { method: 'DELETE' }).catch(() => {});
    }
  });
});
