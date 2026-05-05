import { test, expect, _electron as electron } from '@playwright/test'
import { spawn, ChildProcess } from 'child_process'
import path from 'path'

/**
 * Tauri 桌面应用 E2E 测试
 *
 * 运行前提：
 * 1. cargo install tauri-driver
 * 2. cargo tauri build（生成可执行文件）
 * 3. RECORD_VIDEO=1 npx playwright test tauri-desktop --headed
 */

let tauriDriver: ChildProcess | null = null
const TAURI_BINARY = process.env.TAURI_BINARY ||
  path.join(__dirname, '../../src-tauri/target/release/nx_dashboard')

test.beforeAll(async () => {
  // 启动 tauri-driver（WebDriver 服务器）
  tauriDriver = spawn('tauri-driver', [], {
    stdio: 'inherit'
  })
  await new Promise(resolve => setTimeout(resolve, 2000))
})

test.afterAll(async () => {
  if (tauriDriver) {
    tauriDriver.kill()
  }
})

test.describe('Tauri 桌面应用完整流程', () => {
  test('场景一：ShopFlow 手动驱动（录屏）', async ({ page }) => {
    // 连接到 Tauri 应用
    await page.goto('http://localhost:4444')

    // S1-1: 创建工作区
    await page.click('text=工作区')
    await page.click('text=新建工作区')
    await page.fill('input[name="name"]', 'shopflow')
    await page.fill('input[name="path"]', '/tmp/shopflow')
    await page.click('text=确认')
    await expect(page.locator('text=shopflow')).toBeVisible()

    // S1-2: 上传需求到知识库
    await page.click('text=知识库')
    await page.click('text=上传文档')
    // 文件上传需要特殊处理

    // S1-3: 创建 Workflow
    await page.click('text=工作流')
    await page.click('text=新建')
    await page.fill('input[name="name"]', 'shopflow-build')

    // ... 更多步骤
  })
})
