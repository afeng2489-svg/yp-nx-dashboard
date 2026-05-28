import type { ElementType, ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface PageEmptyStateProps {
  icon: ElementType<{ className?: string }>;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

/** 列表空状态 — 统一 muted 风格 */
export function PageEmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: PageEmptyStateProps) {
  return (
    <div
      className={cn(
        'text-center py-16 rounded-2xl border border-border/50 bg-card',
        className,
      )}
    >
      <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-muted/60 flex items-center justify-center">
        <Icon className="w-8 h-8 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      {description && <p className="text-muted-foreground mb-4">{description}</p>}
      {action}
    </div>
  );
}
