// API configuration
// - Vite 开发（任意 dev 端口）：走 proxy（相对路径），规避 WKWebView 跨端口限制
// - 生产模式（Tauri bundle）：webview origin 是 tauri://localhost，直连 nx_api

/** Playwright/Node 侧 import 时 import.meta.env 可能不存在 */
const viteEnv =
  typeof import.meta !== 'undefined' && import.meta.env
    ? (import.meta.env as ImportMetaEnv)
    : ({} as ImportMetaEnv);

function isViteDevWithProxy(): boolean {
  return (
    viteEnv.DEV &&
    typeof window !== 'undefined' &&
    Boolean(window.location.port) &&
    window.location.port !== '8080'
  );
}

function buildApiBaseUrl(): string {
  const envUrl = viteEnv.VITE_API_BASE_URL as string | undefined;
  if (envUrl) return envUrl;

  if (isViteDevWithProxy()) {
    return '';
  }
  // 生产环境 — 直连后端
  // 使用 127.0.0.1 而非 localhost，避免 Windows 上 localhost 解析为 IPv6 ::1
  // 而后端仅绑定 IPv4 127.0.0.1 导致连接失败
  return 'http://127.0.0.1:8080';
}

export const API_BASE_URL = buildApiBaseUrl();

// WebSocket URL 策略：
// - Vite 开发：走 Vite proxy（同源），避免 WKWebView 跨端口 ws:// 限制
// - 生产模式：直连后端 localhost:8080
function buildWsBaseUrl(): string {
  const envUrl = viteEnv.VITE_WS_BASE_URL as string | undefined;
  if (envUrl) return envUrl;

  if (isViteDevWithProxy() && typeof window !== 'undefined') {
    const { hostname, port } = window.location;
    return `ws://${hostname}:${port}`;
  }
  return 'ws://127.0.0.1:8080';
}

export const WS_BASE_URL = buildWsBaseUrl();
