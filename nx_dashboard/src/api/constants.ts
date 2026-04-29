// API configuration
// - 开发模式 (port 5173)：走 Vite proxy（相对路径），规避 WKWebView 跨端口限制
// - 生产模式（Tauri bundle）：webview origin 是 tauri://localhost，必须用绝对地址直连 nx_api
function buildApiBaseUrl(): string {
  const envUrl = import.meta.env.VITE_API_BASE_URL as string | undefined;
  if (envUrl) return envUrl;

  if (typeof window !== 'undefined') {
    const { port } = window.location;
    if (port === '5173') {
      // Vite dev server — 用相对路径走 proxy
      return '';
    }
  }
  // 生产环境 — 直连后端
  // 使用 127.0.0.1 而非 localhost，避免 Windows 上 localhost 解析为 IPv6 ::1
  // 而后端仅绑定 IPv4 127.0.0.1 导致连接失败
  return 'http://127.0.0.1:8080';
}

export const API_BASE_URL = buildApiBaseUrl();

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
  return 'ws://127.0.0.1:8080';
}

export const WS_BASE_URL = buildWsBaseUrl();
