import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface PageHeaderProps {
  title: string;
  description?: string;
  back?: ReactNode;
  badges?: ReactNode;
  actions?: ReactNode;
  className?: string;
}

/** AF-11 统一页面标题区 */
export function PageHeader({
  title,
  description,
  back,
  badges,
  actions,
  className,
}: PageHeaderProps) {
  return (
    <div className={cn('flex items-start gap-3', className)}>
      {back}
      <div className="min-w-0 flex-1">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h1 className="text-2xl font-semibold tracking-tight text-foreground truncate">
              {title}
            </h1>
            {description && (
              <p className="text-sm text-muted-foreground mt-1">{description}</p>
            )}
          </div>
          {actions && <div className="shrink-0 flex items-center gap-2">{actions}</div>}
        </div>
        {badges && <div className="flex flex-wrap gap-2 mt-2">{badges}</div>}
      </div>
    </div>
  );
}
