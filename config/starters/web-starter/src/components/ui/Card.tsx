import type { ReactNode } from 'react';
import { cn } from '@/lib/cn';

export function Card({
  children,
  className,
  interactive = false,
}: {
  children: ReactNode;
  className?: string;
  interactive?: boolean;
}) {
  return (
    <div
      className={cn(
        'rounded-xl border border-border bg-surface p-6 shadow-soft',
        interactive &&
          'transition-all duration-300 hover:-translate-y-1 hover:border-primary/40 hover:shadow-lift',
        className,
      )}
    >
      {children}
    </div>
  );
}
