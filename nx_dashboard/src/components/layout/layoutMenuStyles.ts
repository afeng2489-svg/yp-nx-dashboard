import { cn } from '@/lib/utils';

/** 顶栏下拉面板 — 不透底，避免与 ContextPanel 等内容叠字 */
export function layoutMenuPanelClassName(width: 'sm' | 'md' = 'md') {
  return cn(
    'absolute right-0 top-full mt-1 z-[100] rounded-xl border border-border',
    'bg-card text-foreground shadow-xl p-1',
    width === 'sm' ? 'w-56' : 'w-64',
  );
}

export function layoutMenuItemClassName(active: boolean) {
  return cn(
    'w-full text-left px-3 py-2.5 rounded-lg transition-colors',
    active ? 'bg-muted' : 'hover:bg-muted/80',
  );
}
