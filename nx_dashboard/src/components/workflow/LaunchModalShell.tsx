import { createPortal } from 'react-dom';
import type { ReactNode } from 'react';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils';

type Accent = 'emerald' | 'indigo' | 'amber';

const ACCENT_DOT: Record<Accent, string> = {
  emerald: 'bg-emerald-500',
  indigo: 'bg-indigo-500',
  amber: 'bg-amber-500',
};

export interface LaunchModalShellProps {
  onClose: () => void;
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  accent?: Accent;
  children: ReactNode;
  footer: ReactNode;
  overlay?: ReactNode;
  size?: 'md' | 'lg';
}

export function LaunchModalShell({
  onClose,
  title,
  subtitle,
  icon,
  accent = 'emerald',
  children,
  footer,
  overlay,
  size = 'lg',
}: LaunchModalShellProps) {
  return createPortal(
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-full items-end justify-center sm:items-center sm:p-6">
        <div
          className="absolute inset-0 bg-black/55 backdrop-blur-[2px]"
          onClick={onClose}
          aria-hidden
        />
        <div
          role="dialog"
          aria-modal
          className={cn(
            'relative flex w-full flex-col overflow-hidden border border-border/80 bg-background shadow-2xl',
            'animate-in fade-in slide-in-from-bottom-4 duration-200 sm:rounded-2xl sm:slide-in-from-bottom-0 sm:zoom-in-95',
            'max-h-[min(92vh,820px)]',
            size === 'lg' ? 'sm:max-w-2xl' : 'sm:max-w-lg',
          )}
        >
          {/* Header — 简洁，少装饰 */}
          <div className="flex shrink-0 items-start gap-3 border-b border-border/60 px-5 py-4 sm:px-6 sm:py-5">
            {icon && (
              <div className="relative mt-0.5 shrink-0 text-muted-foreground [&_svg]:h-5 [&_svg]:w-5">
                {icon}
                <span
                  className={cn(
                    'absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full ring-2 ring-background',
                    ACCENT_DOT[accent],
                  )}
                />
              </div>
            )}
            <div className="min-w-0 flex-1">
              <h2 className="text-base font-semibold leading-snug tracking-tight sm:text-lg">
                {title}
              </h2>
              {subtitle && (
                <p className="mt-1 text-sm leading-relaxed text-muted-foreground">{subtitle}</p>
              )}
            </div>
            <button
              type="button"
              onClick={onClose}
              className="btn-icon -mr-1 shrink-0"
              aria-label="关闭"
            >
              <X className="h-5 w-5" />
            </button>
          </div>

          {/* Body */}
          <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 py-5 sm:px-6 sm:py-6">
            {children}
          </div>

          {/* Footer — 操作区与内容明确分离 */}
          <div className="shrink-0 border-t border-border/60 bg-muted/30 px-5 py-4 sm:px-6">
            {footer}
          </div>

          {overlay}
        </div>
      </div>
    </div>,
    document.body,
  );
}
