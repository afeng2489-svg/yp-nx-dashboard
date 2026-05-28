import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';
import { useShellTheme, useShellVariant } from '@/components/layout/ShellThemeContext';

/** 主内容 Outlet 统一主题容器 — 子页面用 token（bg-card / text-foreground）即可随 Shell 变色 */
export function ShellSurface({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  const theme = useShellTheme();
  const variant = useShellVariant();

  return (
    <div
      data-shell-theme={theme}
      data-shell-variant={variant}
      className={cn(
        'min-h-full',
        /* 深色由 html.dark 统一驱动；此处仅标记 studio 内容区，避免重复强制 dark */
        variant === 'refined' && 'shell-refined',
        className,
      )}
    >
      {children}
    </div>
  );
}
