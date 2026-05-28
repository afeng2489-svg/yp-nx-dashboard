import { createContext, useContext, type ReactNode } from 'react';
import type { LayoutVariant } from '@/data/layoutVariants';
import { normalizeLayoutMode } from '@/data/layoutModes';
import { useThemeStore } from '@/stores/themeStore';
import { useSettingsStore } from '@/stores/settingsStore';

export type ShellTheme = 'light' | 'studio-dark';

interface ShellThemeContextValue {
  theme: ShellTheme;
  variant: LayoutVariant;
}

const ShellThemeContext = createContext<ShellThemeContextValue>({
  theme: 'light',
  variant: 'classic',
});

export function ShellThemeProvider({
  theme,
  variant,
  children,
}: {
  theme: ShellTheme;
  variant: LayoutVariant;
  children: ReactNode;
}) {
  return (
    <ShellThemeContext.Provider value={{ theme, variant }}>
      {children}
    </ShellThemeContext.Provider>
  );
}

export function useShellTheme(): ShellTheme {
  return useContext(ShellThemeContext).theme;
}

export function useShellVariant(): LayoutVariant {
  return useContext(ShellThemeContext).variant;
}

export function useIsStudioDark(): boolean {
  return useShellTheme() === 'studio-dark';
}

/** 布局模式是否为工作室（引导/工作室），与浅/深色主题无关 */
export function useIsStudioLayout(): boolean {
  const mode = useSettingsStore((s) => s.layout.mode);
  return normalizeLayoutMode(mode) === 'studio';
}

/** 跟随设置页 浅/深/系统 主题 */
export function useResolvedShellTheme(): ShellTheme {
  const resolved = useThemeStore((s) => s.resolvedTheme);
  return resolved === 'dark' ? 'studio-dark' : 'light';
}

export function useIsRefinedShell(): boolean {
  return useShellVariant() === 'refined';
}
