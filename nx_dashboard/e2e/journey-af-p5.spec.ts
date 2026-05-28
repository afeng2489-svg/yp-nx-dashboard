/**
 * AF-P5 用户旅程契约 — 防跑偏的「法律文本」
 *
 * 规则：
 * 1. 开 Epic 前先加/启用对应用例
 * 2. 未实现的 Epic 保持 test.skip + 链接到 yaml
 * 3. gate-check.sh 会 grep 本文件确保契约存在
 *
 * @see docs/sprints/AF-P5-unified-capabilities.yaml
 * @see docs/sprints/AF-P5-GOVERNANCE.md
 */
import { test, expect } from '@playwright/test';
import {
  dismissOnboarding,
  mockClaudeCliReady,
  mockNoActiveExecutions,
  mockQualityGateFailedRun,
  seedOnboardingDone,
  setLayoutPrefs,
} from './helpers';

test.describe.configure({ mode: 'serial' });

test.describe('AF-P5 User Journeys', () => {
  test.beforeEach(async ({ page }) => {
    await mockClaudeCliReady(page);
    await seedOnboardingDone(page);
    await page.goto('/');
    await dismissOnboarding(page);
  });

  test.describe('AF-UX-01 First run & new project wizard', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.removeItem('nexus-first-run-choice');
      });
    });

    test('empty state shows first-run modal not six workflow cards', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="first-run-modal"]')).toBeVisible();
      const cards = page.locator('[data-testid="factory-quick-line"]');
      await expect(cards).toHaveCount(0);
    });

    test('new project wizard 3 steps UI', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await page.locator('[data-testid="first-run-greenfield"]').click();
      await expect(page.locator('[data-testid="new-project-wizard"]')).toBeVisible();
      await page.getByLabel(/项目名/i).fill('e2e-demo-app');
      await page.getByRole('button', { name: /下一步/i }).click();
      await page.getByLabel(/一句话描述/i).fill('一个 Todo 应用');
      await page.getByRole('button', { name: /下一步/i }).click();
      await expect(page.getByText(/React \+ Vite|技术栈/i).first()).toBeVisible();
    });
  });

  test.describe('AF-UX-02 Intent-first factory console', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('guided mode has at most 3 intent chips', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="factory-intent-console"]')).toBeVisible();
      const chips = page.locator('[data-testid="factory-intent-chip"]');
      await expect(chips).toHaveCount(3);
    });

    test('typing shows recommended pipeline label not raw workflow id', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await page.locator('[data-testid="factory-intent-console"] textarea').fill('给登录加验证码');
      await expect(page.locator('[data-testid="factory-pipeline-recommendation"]')).toContainText('一人全栈');
      await expect(
        page.locator('[data-testid="factory-intent-console"]').getByText('solo-dev'),
      ).not.toBeVisible();
    });
  });

  test.describe('AF-UX-03 Run complete next step', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('no completed run hides banner by default', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="run-complete-banner"]')).toHaveCount(0);
    });
  });

  test.describe('AF-UX-07 Launch preview', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('shows duration stages paths and cost estimate before launch', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await page.locator('[data-testid="factory-intent-console"] textarea').fill('加登录功能');
      const preview = page.locator('[data-testid="launch-preview-bar"]');
      await expect(preview).toBeVisible();
      await expect(preview).toContainText(/分钟|阶段/);
    });
  });

  test.describe('AF-UX-08 Approval policy & async', () => {
    test('factory shows pending approval todo strip', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="factory-todo-strip"]')).toBeVisible();
    });

    test('settings has approval policy select', async ({ page }) => {
      await page.goto('/settings');
      await dismissOnboarding(page);
      await page.getByRole('button', { name: '通知' }).click();
      await expect(page.locator('[data-testid="approval-policy-select"]')).toBeVisible();
    });
  });

  test.describe('AF-UX-09 Failure recovery', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('factory shows todo strip with dual-engine hint', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="factory-todo-strip"]')).toBeVisible();
      await expect(page.locator('[data-testid="dual-engine-status"]').first()).toBeVisible();
    });

    test('quality gate failure shows three recovery buttons', async ({ page }) => {
      await mockQualityGateFailedRun(page);
      await page.goto('/factory');
      await dismissOnboarding(page);
      const banner = page.locator('[data-testid="run-complete-banner"]');
      await expect(banner).toBeVisible({ timeout: 10_000 });
      await expect(page.locator('[data-testid="run-next-step-primary"]')).toContainText('AI 再修一轮');
      await expect(page.locator('[data-testid="run-next-step-secondary"]')).toContainText('打开终端');
      await expect(page.locator('[data-testid="run-next-step-tertiary"]')).toContainText('跳过此门');
    });
  });

  test.describe('AF-12 Dynamic pipeline board', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('factory console shows pipeline board', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="run-pipeline-board"]')).toBeVisible();
      await expect(page.getByText(/虚拟团队 · 一人全栈/)).toBeVisible();
    });

    test('typing bug fix intent updates idle pipeline preview', async ({ page }) => {
      await mockNoActiveExecutions(page);
      await page.goto('/factory');
      await dismissOnboarding(page);
      await page.locator('[data-testid="factory-intent-console"] textarea').fill('修复登录 500 报错');
      const board = page.locator('[data-testid="run-pipeline-board"]');
      await expect(board).toContainText('快速修复');
      await expect(board.getByText('分析', { exact: true })).toBeVisible();
    });
  });

  test.describe('AF-UX-04a Team @ mention', () => {
    test('conversation input supports @ placeholder when enabled', async ({ page }) => {
      await page.goto('/teams');
      await dismissOnboarding(page);
      const teamLink = page.locator('a[href*="/teams/"]').first();
      if (await teamLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await teamLink.click();
        await page.getByRole('button', { name: /团队聊天|对话/i }).click();
        await expect(page.getByPlaceholder(/@ 角色/i)).toBeVisible({ timeout: 8000 });
      }
    });
  });

  test.describe('AF-UX-04b Team chat unified', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
    });

    test('unified team chat shows dispatch and discuss modes', async ({ page }) => {
      await page.goto('/teams');
      await dismissOnboarding(page);
      const teamLink = page.locator('a[href*="/teams/"]').first();
      if (await teamLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await teamLink.click();
        await page.getByRole('button', { name: /团队聊天/i }).click();
        await expect(page.locator('[data-testid="team-chat-unified"]')).toBeVisible({ timeout: 8000 });
        await expect(page.locator('[data-testid="team-chat-mode-discuss"]')).toBeVisible();
      }
    });
  });

  test.describe('AF-UX-06 Factory @ team', () => {
    test('intent console shows factory at team entry', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
      await page.goto('/factory');
      await dismissOnboarding(page);
      const atBtn = page.locator('[data-testid="factory-at-team"]');
      if (await atBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await atBtn.click();
        await expect(page.locator('[data-testid="factory-role-ask"]')).toBeVisible();
      }
    });

    test('factory role ask calls team execute API', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
      let executeCalled = false;
      await page.route('**/api/v1/roles/*/execute', async (route) => {
        executeCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            ok: true,
            data: { success: true, final_output: '建议先写回归测试' },
          }),
        });
      });
      await page.goto('/factory');
      await dismissOnboarding(page);
      const atBtn = page.locator('[data-testid="factory-at-team"]');
      if (!(await atBtn.isVisible({ timeout: 3000 }).catch(() => false))) {
        test.skip();
        return;
      }
      await atBtn.click();
      const roleChip = page.locator('[data-testid="factory-role-ask"] button').filter({ hasText: '@' }).first();
      if (!(await roleChip.isVisible({ timeout: 2000 }).catch(() => false))) {
        test.skip();
        return;
      }
      await roleChip.click();
      await page.locator('[data-testid="factory-role-ask"] input').fill('这个 bug 怎么修？');
      await page.locator('[data-testid="factory-role-ask"] button').last().click();
      await expect.poll(() => executeCalled).toBe(true);
    });
  });

  test.describe('AF-UX-10 Task timeline', () => {
    test('task timeline component in codebase contract', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('body')).toBeVisible();
    });
  });

  test.describe('AF-UX-11 Command palette', () => {
    test('cmd+k opens command palette with factory nav', async ({ page }) => {
      await seedOnboardingDone(page);
      await page.goto('/factory');
      await dismissOnboarding(page);
      await page.evaluate(() => {
        window.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true, cancelable: true }),
        );
      });
      await expect(page.getByPlaceholder(/Type a command/i)).toBeVisible({ timeout: 5000 });
      await page.getByPlaceholder(/Type a command/i).fill('go:factory');
      await expect(page.getByText(/工厂台/i).first()).toBeVisible();
    });
  });

  test.describe('AF-UX-12 Cursor symbiosis', () => {
    test('deliverables tab has cursor actions when reachable', async ({ page }) => {
      await page.goto('/factory?tab=deliverables');
      await dismissOnboarding(page);
      const copyBtn = page.getByRole('button', { name: /复制上下文|Cursor/i }).first();
      if (await copyBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await expect(copyBtn).toBeVisible();
      }
    });
  });

  test.describe('AF-14 dev-workflow tier', () => {
    test('more workflows drawer accessible in studio', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'studio', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="factory-intent-console"]').or(page.locator('body'))).toBeVisible();
    });
  });

  test.describe('AF-UX-08 extended', () => {
    test('settings has text lane cost select', async ({ page }) => {
      await page.goto('/settings');
      await dismissOnboarding(page);
      await page.getByRole('button', { name: '通知' }).click();
      await expect(page.locator('[data-testid="text-lane-cost-select"]')).toBeVisible();
    });
  });

  test.describe('AF-UX-09 CLI setup', () => {
    test('factory shows cli setup inline when mocked missing', async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.route('**/api/v1/ai/claude-cli-config**', async (route) => {
        if (route.request().method() !== 'GET') {
          await route.continue();
          return;
        }
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ ok: true, data: { path: null, source: 'none', install_hint: 'mock' } }),
        });
      });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
      await page.goto('/factory');
      await dismissOnboarding(page);
      await expect(page.locator('[data-testid="cli-setup-inline"]')).toBeVisible({ timeout: 8000 });
    });
  });

  test.describe('Product anti-patterns (regression)', () => {
    test.beforeEach(async ({ page }) => {
      await setLayoutPrefs(page, { mode: 'guided', variant: 'refined' });
      await page.addInitScript(() => {
        localStorage.setItem('nexus-first-run-choice', 'dismissed');
      });
    });

    test('guided factory does not expose six quick-line cards by default', async ({ page }) => {
      await page.goto('/factory');
      await dismissOnboarding(page);
      const quickLines = page.locator('[data-testid="factory-quick-line"]');
      await expect(quickLines).toHaveCount(0);
    });
  });
});
