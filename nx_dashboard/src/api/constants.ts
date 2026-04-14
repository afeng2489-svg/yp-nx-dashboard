// API configuration
// Use relative URLs - in dev mode Vite proxy handles /api -> http://localhost:8080
// In Tauri production, the API server runs on localhost:8080
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';
export const WS_BASE_URL = import.meta.env.VITE_WS_BASE_URL || '';