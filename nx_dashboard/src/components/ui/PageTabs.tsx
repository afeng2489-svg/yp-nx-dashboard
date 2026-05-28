import type { ElementType } from 'react';
import { cn } from '@/lib/utils';

export interface PageTabItem {
  id: string;
  label: string;
  icon?: ElementType<{ className?: string }>;
}

interface PageTabsProps {
  items: PageTabItem[];
  value: string;
  onValueChange: (id: string) => void;
  variant?: 'underline' | 'pills';
  className?: string;
}

/** AF-11 统一 Tab 导航（underline 主 Tab / pills 子 Tab） */
export function PageTabs({
  items,
  value,
  onValueChange,
  variant = 'underline',
  className,
}: PageTabsProps) {
  if (variant === 'pills') {
    return (
      <div className={cn('flex flex-wrap gap-1', className)}>
        {items.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onValueChange(item.id)}
            className={cn(
              'px-2.5 py-1 text-xs rounded-md transition-colors',
              value === item.id
                ? 'bg-primary/10 text-primary font-medium'
                : 'text-muted-foreground hover:bg-accent/50',
            )}
          >
            {item.label}
          </button>
        ))}
      </div>
    );
  }

  return (
    <div className={cn('flex gap-1 border-b border-border/60 overflow-x-auto', className)}>
      {items.map((item) => {
        const Icon = item.icon;
        return (
          <button
            key={item.id}
            type="button"
            onClick={() => onValueChange(item.id)}
            className={cn(
              'inline-flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium whitespace-nowrap border-b-2 -mb-px transition-colors',
              value === item.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground',
            )}
          >
            {Icon && <Icon className="w-4 h-4" />}
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
