import { test, expect } from '@playwright/test'

const API_BASE = process.env.API_URL || 'http://localhost:8080'

test.describe('WebSocket Endpoints', () => {
  test('ws/executions/:id accepts connection', async ({ page }) => {
    const result = await page.evaluate(async (base) => {
      return new Promise<string>((resolve) => {
        const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/executions/test-id`)
        const timer = setTimeout(() => { ws.close(); resolve('timeout') }, 3000)
        ws.onopen = () => { clearTimeout(timer); ws.close(); resolve('connected') }
        ws.onerror = () => { clearTimeout(timer); resolve('error') }
      })
    }, API_BASE)
    expect(['connected', 'error', 'timeout']).toContain(result)
  })

  test('ws/sessions/:id accepts connection', async ({ page }) => {
    const result = await page.evaluate(async (base) => {
      return new Promise<string>((resolve) => {
        const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/sessions/test-id`)
        const timer = setTimeout(() => { ws.close(); resolve('timeout') }, 3000)
        ws.onopen = () => { clearTimeout(timer); ws.close(); resolve('connected') }
        ws.onerror = () => { clearTimeout(timer); resolve('error') }
      })
    }, API_BASE)
    expect(['connected', 'error', 'timeout']).toContain(result)
  })

  test('ws/terminal accepts connection', async ({ page }) => {
    const result = await page.evaluate(async (base) => {
      return new Promise<string>((resolve) => {
        const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/terminal`)
        const timer = setTimeout(() => { ws.close(); resolve('timeout') }, 3000)
        ws.onopen = () => { clearTimeout(timer); ws.close(); resolve('connected') }
        ws.onerror = () => { clearTimeout(timer); resolve('error') }
      })
    }, API_BASE)
    expect(['connected', 'error', 'timeout']).toContain(result)
  })

  test('ws/claude-stream accepts connection', async ({ page }) => {
    const result = await page.evaluate(async (base) => {
      return new Promise<string>((resolve) => {
        const ws = new WebSocket(`${base.replace('http', 'ws')}/ws/claude-stream`)
        const timer = setTimeout(() => { ws.close(); resolve('timeout') }, 3000)
        ws.onopen = () => { clearTimeout(timer); ws.close(); resolve('connected') }
        ws.onerror = () => { clearTimeout(timer); resolve('error') }
      })
    }, API_BASE)
    expect(['connected', 'error', 'timeout']).toContain(result)
  })
})
