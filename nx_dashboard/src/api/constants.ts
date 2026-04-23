// API configuration
// Use relative URLs - in dev mode Vite proxy handles /api -> http://localhost:8080
// In Tauri production, the API server runs on localhost:8080
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

// WebSocket URL 策略：
// - 开发模式 (port 5173)：走 Vite proxy（同源），避免 WKWebView 跨端口限制
// - 生产模式：直连后端 localhost:8080
function buildWsBaseUrl(): string {
  const envUrl = import.meta.env.VITE_WS_BASE_URL as string | undefined;
  if (envUrl) return envUrl;

  if (typeof window !== 'undefined') {
    const { hostname, port } = window.location;
    if (port === '5173') {
      // Vite dev server — 走 proxy，规避 WKWebView 跨端口 ws:// 限制
      return `ws://${hostname}:5173`;
    }
  }
  // 生产环境（Tauri bundle）— 直连后端
  return 'ws://localhost:8080';
}

export const WS_BASE_URL = buildWsBaseUrl();
