import { AppShell } from './AppShell';

/** @deprecated 使用 AppShell；保留别名便于测试/文档引用 */
export const GuidedShellClassic = AppShell;
export const StudioShellClassic = AppShell;
export const GuidedShellRefined = AppShell;
export const StudioShellRefined = AppShell;

export { AppShell };

export function resolveAppShell() {
  return AppShell;
}
