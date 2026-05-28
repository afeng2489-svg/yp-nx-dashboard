import type { ElementType, ReactNode } from 'react';
import { Sparkles, X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface PageGuideBannerProps {
  title: string;
  children: ReactNode;
  icon?: ElementType<{ className?: string }>;
  onDismiss?: () => void;
  className?: string;
}

/** 页面操作指南条 — 使用 token，精炼/Studio 下与 Shell 一致 */
export function PageGuideBanner({
  title,
  children,
  icon: Icon = Sparkles,
  onDismiss,
  className,
}: PageGuideBannerProps) {
  return (
    <div className={cn('rounded-2xl border border-border/60 bg-muted/40 p-5 relative', className)}>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          className="absolute top-3 right-3 p-1.5 hover:bg-accent rounded-lg transition-colors"
          aria-label="关闭"
        >
          <X className="w-4 h-4 text-muted-foreground" />
        </button>
      )}
      <div className="flex items-start gap-4">
        <div className="p-2.5 rounded-xl bg-primary/10 text-primary shrink-0">
          <Icon className="w-5 h-5" />
        </div>
        <div className="min-w-0">
          <p className="font-semibold mb-2">{title}</p>
          {children}
        </div>
      </div>
    </div>
  );
}
