import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

export interface DiscussionEmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
  compact?: boolean;
}

export function DiscussionEmptyState({
  icon,
  title,
  description,
  action,
  compact = false,
}: DiscussionEmptyStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center text-center',
        compact ? 'py-10 px-4' : 'py-16 px-6',
      )}
    >
      <div
        className={cn(
          'relative flex items-center justify-center rounded-2xl bg-muted/40 ring-1 ring-border/50',
          compact ? 'mb-3 h-12 w-12' : 'mb-5 h-16 w-16',
        )}
      >
        <span className="text-muted-foreground [&_svg]:h-6 [&_svg]:w-6">{icon}</span>
        <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full bg-indigo-500 ring-2 ring-background" />
      </div>
      <p className={cn('font-medium text-foreground', compact ? 'text-sm' : 'text-base')}>
        {title}
      </p>
      <p
        className={cn(
          'mt-1 max-w-xs leading-relaxed text-muted-foreground',
          compact ? 'text-xs' : 'text-sm',
        )}
      >
        {description}
      </p>
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
