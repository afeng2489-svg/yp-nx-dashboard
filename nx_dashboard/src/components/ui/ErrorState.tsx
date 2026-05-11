import { AlertCircle, Inbox, WifiOff, Search, RotateCw, Lightbulb } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ComponentType } from 'react';

interface ErrorAction {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'secondary';
}

interface ErrorStateProps {
  variant?: 'error' | 'empty' | 'network' | 'notfound';
  title: string;
  message?: string;
  hints?: string[];
  actions?: ErrorAction[];
  className?: string;
  compact?: boolean;
}

const VARIANT_CONFIG: Record<
  NonNullable<ErrorStateProps['variant']>,
  { icon: ComponentType<{ className?: string }>; color: string; bg: string }
> = {
  error: {
    icon: AlertCircle,
    color: 'text-red-500',
    bg: 'from-red-500/10 to-rose-500/10',
  },
  empty: {
    icon: Inbox,
    color: 'text-muted-foreground',
    bg: 'from-muted/30 to-muted/10',
  },
  network: {
    icon: WifiOff,
    color: 'text-amber-500',
    bg: 'from-amber-500/10 to-orange-500/10',
  },
  notfound: {
    icon: Search,
    color: 'text-sky-500',
    bg: 'from-sky-500/10 to-cyan-500/10',
  },
};

/**
 * 统一的错误 / 空状态展示组件
 *
 * 使用场景：
 * - 加载失败：variant="error"，带重试
 * - 空数据：variant="empty"，带"创建第一个"
 * - 网络不通：variant="network"，给出诊断步骤
 * - 404：variant="notfound"，返回列表
 */
export function ErrorState({
  variant = 'error',
  title,
  message,
  hints,
  actions,
  className,
  compact = false,
}: ErrorStateProps) {
  const config = VARIANT_CONFIG[variant];
  const Icon = config.icon;

  return (
    <div
      className={cn(
        'flex flex-col items-center text-center rounded-2xl border border-border/50',
        'bg-gradient-to-b',
        config.bg.replace('/10', '/5'),
        compact ? 'py-8 px-4' : 'py-12 px-6',
        className,
      )}
    >
      <div
        className={cn(
          'rounded-2xl flex items-center justify-center mb-4 bg-gradient-to-br',
          config.bg,
          compact ? 'w-12 h-12' : 'w-16 h-16',
        )}
      >
        <Icon className={cn(config.color, compact ? 'w-6 h-6' : 'w-8 h-8')} />
      </div>
      <h3 className={cn('font-semibold', compact ? 'text-sm' : 'text-lg', 'mb-1')}>{title}</h3>
      {message && (
        <p
          className={cn(
            'text-muted-foreground max-w-md',
            compact ? 'text-xs' : 'text-sm',
            hints || actions ? 'mb-4' : '',
          )}
        >
          {message}
        </p>
      )}
      {hints && hints.length > 0 && (
        <div className="text-left bg-background/50 rounded-xl border border-border/50 px-4 py-3 mb-4 max-w-md w-full">
          <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground mb-2">
            <Lightbulb className="w-3 h-3" />
            可能的原因
          </div>
          <ul className="text-xs text-muted-foreground space-y-1 list-disc list-inside">
            {hints.map((h, i) => (
              <li key={i}>{h}</li>
            ))}
          </ul>
        </div>
      )}
      {actions && actions.length > 0 && (
        <div className="flex items-center gap-2 flex-wrap justify-center">
          {actions.map((a, i) => (
            <button
              key={i}
              onClick={a.onClick}
              className={cn(
                'px-4 py-2 rounded-xl text-sm font-medium transition-all',
                a.variant === 'primary' || (i === 0 && !a.variant)
                  ? 'bg-primary text-primary-foreground hover:bg-primary/90 shadow-md'
                  : 'bg-card border border-border/50 hover:bg-accent',
              )}
            >
              {i === 0 && a.label.includes('重试') && (
                <RotateCw className="w-3.5 h-3.5 inline mr-1" />
              )}
              {a.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
