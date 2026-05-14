import { test, expect } from '@playwright/test';

/**
 * Tauri 桌面应用 E2E 测试（dev 模式）
 *
 * 运行前提：cargo tauri dev 已启动
 * 测试通过 Tauri webview 的 dev URL 进行交互
 */

const TAURI_URL = process.env.TAURI_URL || 'http://localhost:5174';

test.describe.configure({ mode: 'serial' });

test.describe('Tauri 桌面应用', () => {
  test('应用加载', async ({ page }) => {
    await page.goto(TAURI_URL);
    await expect(page.locator('body')).toBeVisible({ timeout: 15000 });
  });

  test('侧边栏导航可见', async ({ page }) => {
    await page.goto(TAURI_URL);
    // 等待侧边栏渲染
    const nav = page
      .locator('nav, [class*="sidebar"], [class*="Sidebar"], [class*="menu"]')
      .first();
    await expect(nav).toBeVisible({ timeout: 10000 });
  });

  test('创建工作区', async ({ page }) => {
    await page.goto(TAURI_URL);
    // 点击工作区相关导航
    const workspaceNav = page.locator('text=工作区').or(page.locator('text=Workspace')).first();
    if (await workspaceNav.isVisible()) {
      await workspaceNav.click();
    }
    // 检查页面是否正常响应
    await expect(page.locator('body')).toBeVisible();
  });
});
