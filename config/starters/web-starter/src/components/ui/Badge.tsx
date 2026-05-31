import type { ReactNode } from 'react';
import { cn } from '@/lib/cn';

export function Badge({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full border border-border bg-surface/70 px-3 py-1',
        'text-xs font-semibold uppercase tracking-wider text-muted backdrop-blur',
        className,
      )}
    >
      {children}
    </span>
  );
}
