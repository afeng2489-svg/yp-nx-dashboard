// API configuration
// Use relative URLs - in dev mode Vite proxy handles /api -> http://localhost:8080
// In Tauri production, the API server should be accessible directly at the same origin
// or use tauri-plugin-http for HTTP requests
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';
export const WS_BASE_URL = import.meta.env.VITE_WS_BASE_URL || '';