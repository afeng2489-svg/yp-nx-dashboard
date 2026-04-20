// API configuration
// Use relative URLs - in dev mode Vite proxy handles /api -> http://localhost:8080
// In Tauri production, the API server runs on localhost:8080
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

// WebSocket 必须使用绝对 URL（ws:// 或 wss://）
// 如果未配置 VITE_WS_BASE_URL，则从当前页面地址推导
function buildWsBaseUrl(): string {
  const envUrl = import.meta.env.VITE_WS_BASE_URL as string | undefined;
  if (envUrl) return envUrl;
  // 浏览器环境：从 window.location 推导
  if (typeof window !== 'undefined') {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}`;
  }
  return 'ws://localhost:8080';
}

export const WS_BASE_URL = buildWsBaseUrl();
