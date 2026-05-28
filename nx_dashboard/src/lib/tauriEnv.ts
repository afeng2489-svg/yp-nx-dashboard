/** 是否在 Tauri 桌面壳内运行（非纯浏览器） */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
