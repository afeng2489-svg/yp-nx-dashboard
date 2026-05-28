/**
 * GATE-1: Golden Path E2E
 * 无 Claude CLI 时 skip；有 CLI 时启动 Run 并轮询产物（最长 15min，默认 smoke 仅验证启动）。
 */
import { test, expect } from '@playwright/test';
import { API_BASE, api } from './helpers';
import { GOLDEN_PATH_TASK } from '../src/services/factoryMetrics';

async function cliAvailable(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/api/v1/ai/claude-cli-config`);
    if (!res.ok) return false;
    const body = await res.json();
    const data = body.data ?? body;
    return data.source !== 'none' && !!data.path;
  } catch {
    return false;
  }
}

test.describe('Golden Path', () => {
  test('GATE-1: factory metrics + golden run smoke', async () => {
    const hasCli = await cliAvailable();
    test.skip(!hasCli, 'Claude CLI 未配置 — 见 docs/GOLDEN-PATH.md');

    const metricsRes = await fetch(`${API_BASE}/api/v1/factory/metrics`);
    expect(metricsRes.status).toBe(200);
    const metrics = await metricsRes.json();
    expect(metrics).toHaveProperty('activation');
    expect(metrics).toHaveProperty('golden_path_success');

    const teamRes = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ template: 'solo-fullstack', name: `gp-e2e-${Date.now()}` }),
    });
    expect(teamRes.status).toBe(200);
    const teamBody = await teamRes.json();
    const teamId = teamBody.data?.team_id ?? teamBody.team_id;

    const runRes = await fetch(`${API_BASE}/api/v1/quick-run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prompt: GOLDEN_PATH_TASK,
        team_id: teamId,
        workflow_name: 'solo-dev',
      }),
    });
    expect(runRes.status).toBe(200);
    const runBody = await runRes.json();
    expect(runBody.ok).toBe(true);
    const execId = runBody.data.execution_id as string;
    expect(execId).toBeTruthy();

    await fetch(`${API_BASE}/api/v1/factory/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        device_id: 'e2e-golden-path',
        event_type: 'run_started',
        execution_id: execId,
        payload: { golden_path: true },
      }),
    });

    const ex = (await api(`/api/v1/executions/${execId}`)) as { status: string };
    expect(['pending', 'running', 'paused']).toContain(ex.status);

    await fetch(`${API_BASE}/api/v1/executions/${execId}/cancel`, { method: 'POST' });
  });
});
