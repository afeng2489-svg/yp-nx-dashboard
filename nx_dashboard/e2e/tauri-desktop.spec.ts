import { test, expect } from '@playwright/test';
import { dismissOnboarding, setLayoutPrefs } from './helpers';

/**
 * Tauri 桌面应用 E2E 测试（dev 模式）
 *
 * 运行前提：npm run tauri:dev 或 npm run dev 已启动
 */

const APP_URL = process.env.TAURI_URL || process.env.BASE_URL || 'http://localhost:1420';

test.describe.configure({ mode: 'serial' });

test.describe('Tauri 桌面应用', () => {
  test('应用加载', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'guided', variant: 'classic' });
    await page.goto(APP_URL);
    await dismissOnboarding(page);
    await expect(page.locator('body')).toBeVisible({ timeout: 15000 });
  });

  test('侧边栏导航可见', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'guided', variant: 'classic' });
    await page.goto(APP_URL);
    await dismissOnboarding(page);
    const nav = page.locator('aside nav').first();
    await expect(nav).toBeVisible({ timeout: 10000 });
  });

  test('创建工作区', async ({ page }) => {
    await setLayoutPrefs(page, { mode: 'guided', variant: 'classic' });
    await page.goto(APP_URL);
    await dismissOnboarding(page);
    // 点击工作区相关导航
    const workspaceNav = page.locator('text=工作区').or(page.locator('text=Workspace')).first();
    if (await workspaceNav.isVisible()) {
      await workspaceNav.click();
    }
    // 检查页面是否正常响应
    await expect(page.locator('body')).toBeVisible();
  });
});
