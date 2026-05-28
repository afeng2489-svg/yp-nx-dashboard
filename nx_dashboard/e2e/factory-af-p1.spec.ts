/**
 * AF-P1 端到端冒烟：工厂台 MVP + 审批 API + WS/poll 字段
 * 不跑完整 solo-dev CLI（太慢），验证关键 API/WS 契约。
 */
import { test, expect } from '@playwright/test';
import {
  api,
  API_BASE,
  dismissOnboarding,
  openLayoutSettings,
  pickLayoutMode,
  pickLayoutVariant,
  seedOnboardingDone,
  setLayoutPrefs,
} from './helpers';

test.describe.configure({ mode: 'serial' });

test.describe('AF-P1 Factory smoke', () => {
  let teamId: string;
  let executionId: string;

  test('F5: POST /teams/from-template solo-fullstack', async () => {
    const res = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        template: 'solo-fullstack',
        name: `e2e-solo-${Date.now()}`,
      }),
    });
    expect(res.status, 'from-template 应已部署（非 405）').toBe(200);
    const body = await res.json();
    const data = body.data ?? body;
    teamId = data.team_id ?? data.team?.id;
    expect(teamId).toBeTruthy();
  });

  test('F5: bad template → 4xx', async () => {
    const res = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ template: 'nonexistent-template' }),
    });
    expect(res.status).toBeGreaterThanOrEqual(400);
  });

  test('F6: quick-run 带 factory 元数据', async () => {
    const res = await fetch(`${API_BASE}/api/v1/quick-run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prompt: 'e2e smoke: 验证工厂台 quick-run',
        team_id: teamId,
        workflow_name: 'solo-dev',
      }),
    });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.ok).toBe(true);
    executionId = body.data.execution_id;
    expect(executionId).toBeTruthy();

    const ex = (await api(`/api/v1/executions/${executionId}`)) as {
      id: string;
      status: string;
      trigger_source?: string;
      team_id?: string;
      current_stage?: string | null;
    };
    expect(ex.id).toBe(executionId);
    expect(ex.trigger_source).toBe('factory');
    expect(ex.team_id).toBe(teamId);
    expect(['pending', 'running', 'paused']).toContain(ex.status);
    expect('current_stage' in ex).toBe(true);
  });

  test('F4: artifacts summary 端点可用', async () => {
    const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/artifacts/summary`);
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.ok ?? true).toBeTruthy();
  });

  test('W1/W3: WS 连接收到 snapshot 或 stage 事件', async ({ page }) => {
    const eventTypes = await page.evaluate(
      async ({ base, execId }) => {
        return new Promise<string[]>((resolve) => {
          const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/executions/${execId}`);
          const types: string[] = [];
          const done = () => {
            ws.close();
            resolve(types);
          };
          const timer = setTimeout(done, 12_000);
          ws.onmessage = (ev) => {
            try {
              const d = JSON.parse(ev.data as string) as { type?: string };
              if (d.type) types.push(d.type);
              if (
                d.type === 'snapshot' ||
                d.type === 'stage_started' ||
                d.type === 'started' ||
                d.type === 'output'
              ) {
                clearTimeout(timer);
                setTimeout(done, 300);
              }
            } catch {
              /* ignore */
            }
          };
          ws.onerror = () => {
            clearTimeout(timer);
            done();
          };
        });
      },
      { base: API_BASE, execId: executionId },
    );

    expect(
      eventTypes.some((t) =>
        ['snapshot', 'started', 'stage_started', 'output', 'stage_completed'].includes(t),
      ),
      `WS 应推送事件，收到: ${eventTypes.join(', ') || '(none)'}`,
    ).toBe(true);
  });

  test('W2: GET poll 在 WS 断开后仍能读到 execution', async ({ page }) => {
    await page.evaluate(
      async ({ base, execId }) => {
        await new Promise<void>((resolve) => {
          const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/executions/${execId}`);
          ws.onopen = () => ws.close();
          ws.onclose = () => resolve();
          ws.onerror = () => resolve();
          setTimeout(resolve, 2000);
        });
      },
      { base: API_BASE, execId: executionId },
    );

    await new Promise((r) => setTimeout(r, 500));
    const a = (await api(`/api/v1/executions/${executionId}`)) as { status: string };
    await new Promise((r) => setTimeout(r, 1500));
    const b = (await api(`/api/v1/executions/${executionId}`)) as {
      status: string;
      current_stage?: string | null;
    };
    expect(a.status).toBeTruthy();
    expect(b.status).toBeTruthy();
    expect('current_stage' in b).toBe(true);
  });

  test('A1: resolve 非 paused 执行 → 4xx', async () => {
    const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/resolve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ approved: true, comment: 'e2e should fail' }),
    });
    expect(res.status).toBeGreaterThanOrEqual(400);
  });

  test('cleanup: cancel execution', async () => {
    const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/cancel`, {
      method: 'POST',
    });
    expect([200, 204, 404]).toContain(res.status);
  });
});

test.describe('AF-10 IA smoke', () => {
  test('TeamDetailPage route: /teams/:id?tab=discuss not 404', async ({ page }) => {
    const teamsRes = await fetch(`${API_BASE}/api/v1/teams`);
    expect(teamsRes.ok).toBe(true);
    const teamsBody = await teamsRes.json();
    const teams = teamsBody.data ?? teamsBody;
    const first = Array.isArray(teams) ? teams[0] : null;
    test.skip(!first?.id, 'no teams to test TeamDetailPage');

    const res = await page.goto(`/teams/${first.id}?tab=discuss`);
    expect(res?.status()).toBeLessThan(400);
    await expect(page.locator('body')).not.toContainText('404');
  });

  test('Assets knowledge RAG sub-tab renders', async ({ page }) => {
    const res = await page.goto('/assets?tab=knowledge&sub=rag');
    expect(res?.status()).toBeLessThan(400);
    await expect(page.getByText('知识库').first()).toBeVisible({ timeout: 15_000 });
  });

  test('Legacy /wisdom redirects to wisdom sub-tab', async ({ page }) => {
    await page.goto('/wisdom');
    await page.waitForURL(/tab=knowledge.*sub=wisdom|sub=wisdom.*tab=knowledge/);
    expect(page.url()).toContain('sub=wisdom');
  });

  test('Canvas editor route reachable', async ({ page }) => {
    const res = await page.goto('/canvas');
    expect(res?.status()).toBeLessThan(400);
    expect(page.url()).toContain('/canvas');
  });

  test('Factory attachment upload API', async () => {
    const form = new FormData();
    form.append('file', new Blob(['e2e attachment content'], { type: 'text/plain' }), 'e2e.txt');
    const res = await fetch(`${API_BASE}/api/v1/factory/attachments`, {
      method: 'POST',
      body: form,
    });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.data?.relative_path).toContain('.nx/factory-attachments/');
    expect(body.data?.text_excerpt).toContain('e2e attachment');
  });
});

test.describe('AF-11 layout modes', () => {
  test.beforeEach(async ({ page }) => {
    await seedOnboardingDone(page);
  });

  test('settings layout tab shows mode + variant pickers', async ({ page }) => {
    await openLayoutSettings(page);
    await expect(page.getByText('引导模式').first()).toBeVisible();
    await expect(page.getByText('工作室模式').first()).toBeVisible();
    await expect(page.getByText('专注模式')).toHaveCount(0);
    await expect(page.getByText('界面风格').first()).toBeVisible();
    await expect(page.getByText('经典界面').first()).toBeVisible();
    await expect(page.getByText('精炼界面').first()).toBeVisible();
  });

  test('studio refined: ops runs tab + file sidebar tabs', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'studio', variant: 'refined' });
    await page.goto('/ops?tab=runs');
    await dismissOnboarding(page);
    await expect(page.getByRole('heading', { name: '运营' })).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('button', { name: '历史' })).toBeVisible();
    await expect(page.getByRole('button', { name: '变更' })).toBeVisible();
    await expect(page.getByText('文件').first()).toBeVisible();
  });

  test('factory console shows solo team pipeline board', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
    await page.goto('/factory?tab=console');
    await dismissOnboarding(page);
    await expect(page.getByText('虚拟团队 · 一人全栈')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('规划').first()).toBeVisible();
    await expect(page.getByText('交付审批').first()).toBeVisible();
  });

  test('classic variant restores guided shell with sidebar', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'guided', variant: 'classic' });
    await page.goto('/factory');
    await dismissOnboarding(page);
    await expect(page.locator('aside').first()).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('heading', { name: '工厂台' })).toBeVisible();
  });
});
