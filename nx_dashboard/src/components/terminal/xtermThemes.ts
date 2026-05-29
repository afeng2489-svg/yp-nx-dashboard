import type { ITheme } from '@xterm/xterm';

/** xterm 深色主题（zinc-900，对齐 --card，避免比外壳更黑一块） */
export const XTERM_THEME_DARK: ITheme = {
  background: '#18181b',
  foreground: '#e4e4e7',
  cursor: '#fafafa',
  cursorAccent: '#18181b',
  selectionBackground: '#3f3f46',
  black: '#18181b',
  red: '#f87171',
  green: '#4ade80',
  yellow: '#facc15',
  blue: '#60a5fa',
  magenta: '#c084fc',
  cyan: '#22d3ee',
  white: '#e4e4e7',
  brightBlack: '#71717a',
  brightRed: '#fca5a5',
  brightGreen: '#86efac',
  brightYellow: '#fde047',
  brightBlue: '#93c5fd',
  brightMagenta: '#d8b4fe',
  brightCyan: '#67e8f9',
  brightWhite: '#fafafa',
};

/** xterm 浅色主题（对齐设置页浅色 / VS Code light terminal） */
export const XTERM_THEME_LIGHT: ITheme = {
  background: '#ffffff',
  foreground: '#18181b',
  cursor: '#18181b',
  cursorAccent: '#ffffff',
  selectionBackground: '#bfdbfe',
  black: '#18181b',
  red: '#dc2626',
  green: '#16a34a',
  yellow: '#ca8a04',
  blue: '#2563eb',
  magenta: '#9333ea',
  cyan: '#0891b2',
  white: '#fafafa',
  brightBlack: '#71717a',
  brightRed: '#ef4444',
  brightGreen: '#22c55e',
  brightYellow: '#eab308',
  brightBlue: '#3b82f6',
  brightMagenta: '#a855f7',
  brightCyan: '#06b6d4',
  brightWhite: '#ffffff',
};

export function xtermThemeFor(resolved: 'light' | 'dark'): ITheme {
  return resolved === 'dark' ? XTERM_THEME_DARK : XTERM_THEME_LIGHT;
}
