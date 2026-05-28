import { expect } from '@playwright/test';

export const API_BASE = process.env.API_URL || 'http://localhost:8080';

export function unwrap(body: unknown): unknown {
  if (body && typeof body === 'object' && 'ok' in (body as Record<string, unknown>)) {
    const env = body as { ok: boolean; data?: unknown; error?: string };
    if (!env.ok) throw new Error(env.error ?? 'API error');
    return env.data;
  }
  return body;
}

export async function api(path: string, opts: RequestInit = {}): Promise<unknown> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  });
  const body = await res.json();
  if (!res.ok) throw new Error(body?.error ?? body?.message ?? `HTTP ${res.status}`);
  return unwrap(body);
}

export async function createWorkflow(name = `e2e-wf-${Date.now()}`) {
  return api('/api/v1/workflows', {
    method: 'POST',
    body: JSON.stringify({
      name,
      description: 'e2e test',
      definition: { stages: [{ name: 's1', prompt: 'hello' }] },
    }),
  }) as Promise<{ id: string }>;
}

export async function deleteWorkflow(id: string) {
  await fetch(`${API_BASE}/api/v1/workflows/${id}`, { method: 'DELETE' }).catch(() => {});
}

export async function expectOk(res: Response) {
  expect([200, 201, 204]).toContain(res.status);
}

/** E2E：取消活跃 execution，避免产线看板被旧 Run 占用 */
export async function cancelActiveExecutions() {
  try {
    const list = (await api('/api/v1/executions')) as Array<{ id: string; status: string }>;
    const active = (Array.isArray(list) ? list : []).filter((e) =>
      ['running', 'pending', 'paused'].includes(e.status),
    );
    await Promise.all(
      active.map((e) =>
        fetch(`${API_BASE}/api/v1/executions/${e.id}/cancel`, { method: 'POST' }).catch(() => {}),
      ),
    );
  } catch {
    /* API 不可用时跳过 */
  }
}

/** E2E：隐藏活跃 Run，让产线看板走 idle 预览（不触发 CLI） */
export async function mockNoActiveExecutions(page: import('@playwright/test').Page) {
  let filtered: unknown[] = [];
  try {
    const list = (await api('/api/v1/executions')) as Array<{ status: string }>;
    filtered = (Array.isArray(list) ? list : []).filter(
      (e) => !['running', 'pending', 'paused'].includes(e.status),
    );
  } catch {
    filtered = [];
  }

  await page.route('**/api/v1/executions', async (route) => {
    const url = new URL(route.request().url());
    if (route.request().method() !== 'GET' || url.pathname !== '/api/v1/executions') {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(filtered),
    });
  });
}

/** E2E：模拟质量门失败的最近工厂 Run，用于 AF-UX-09 三按钮 Banner */
export async function mockQualityGateFailedRun(
  page: import('@playwright/test').Page,
  workflowId = 'wf-solo-dev',
) {
  const now = Date.now();
  const failedExec = {
    id: 'exec-qg-fail-e2e',
    workflow_id: workflowId,
    status: 'failed',
    trigger_source: 'factory',
    finished_at: new Date(now).toISOString(),
    started_at: new Date(now - 60_000).toISOString(),
    variables: { task: 'fix failing tests' },
    stage_results: [
      {
        stage_name: '测试',
        quality_gate_result: {
          passed: false,
          retry_count: 2,
          checks: [
            {
              cmd: 'cargo test',
              passed: false,
              exit_code: 1,
              stdout: '',
              stderr: 'fail',
              duration_ms: 100,
            },
          ],
        },
      },
    ],
    error: '质量门未通过',
  };

  await page.route('**/api/v1/executions', async (route) => {
    const url = new URL(route.request().url());
    if (route.request().method() !== 'GET' || url.pathname !== '/api/v1/executions') {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ ok: true, data: [failedExec] }),
    });
  });

  await page.route('**/api/v1/workflows', async (route) => {
    if (route.request().method() !== 'GET') {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        ok: true,
        data: [
          {
            id: workflowId,
            name: 'solo-dev',
            version: '1.0.0',
            stage_count: 3,
            agent_count: 1,
          },
        ],
      }),
    });
  });
}

/** E2E：模拟 Claude CLI 已就绪，避免真实调用消耗 token */
export async function mockClaudeCliReady(page: import('@playwright/test').Page) {
  await page.route('**/api/v1/ai/claude-cli-config**', async (route) => {
    if (route.request().method() !== 'GET') {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        ok: true,
        data: {
          path: '/usr/local/bin/claude',
          source: 'auto',
          install_hint: null,
        },
      }),
    });
  });
}

/** 关闭 onboarding 向导（若弹出） */
export async function dismissOnboarding(page: import('@playwright/test').Page) {
  const skip = page.getByRole('button', { name: '跳过' });
  if (await skip.isVisible({ timeout: 1500 }).catch(() => false)) {
    await skip.click({ force: true });
    await skip.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  }
}

/** 设置 localStorage，避免 onboarding 遮挡 UI 测试 */
export async function seedOnboardingDone(page: import('@playwright/test').Page) {
  await page.addInitScript(() => {
    localStorage.setItem('nexus-onboarding-v1', 'done');
  });
}

type LayoutMode = 'guided' | 'studio';
type LayoutVariant = 'classic' | 'refined';

/** 通过 zustand persist 直接设置布局（比点选设置页更稳） */
export async function setLayoutPrefs(
  page: import('@playwright/test').Page,
  prefs: { mode?: LayoutMode; variant?: LayoutVariant },
) {
  await seedOnboardingDone(page);
  await page.addInitScript((patch) => {
    const key = 'nexus-settings';
    const raw = localStorage.getItem(key);
    const parsed = raw ? JSON.parse(raw) : { state: { layout: {} }, version: 0 };
    parsed.state = parsed.state ?? {};
    parsed.state.layout = { ...(parsed.state.layout ?? {}), ...patch };
    localStorage.setItem(key, JSON.stringify(parsed));
  }, prefs);
}

/** 设置页 → 布局 Tab */
export async function openLayoutSettings(page: import('@playwright/test').Page) {
  await seedOnboardingDone(page);
  await page.goto('/settings');
  await dismissOnboarding(page);
  await page.getByRole('button', { name: '布局', exact: true }).click();
  await page.getByText('布局模式').first().waitFor({ state: 'visible', timeout: 10_000 });
}

function layoutModeSection(page: import('@playwright/test').Page) {
  return page
    .locator('div')
    .filter({ has: page.getByText('布局模式', { exact: true }) })
    .filter({ has: page.getByRole('button', { name: /引导模式/ }) });
}

function layoutVariantSection(page: import('@playwright/test').Page) {
  return page
    .locator('div')
    .filter({ has: page.getByText('界面风格', { exact: true }) })
    .filter({ has: page.getByRole('button', { name: /经典界面/ }) });
}

export async function pickLayoutMode(
  page: import('@playwright/test').Page,
  label: '引导模式' | '工作室模式',
) {
  await layoutModeSection(page).getByRole('button', { name: new RegExp(label) }).click();
}

export async function pickLayoutVariant(
  page: import('@playwright/test').Page,
  label: '经典界面' | '精炼界面',
) {
  await layoutVariantSection(page).getByRole('button', { name: new RegExp(label) }).click();
}
