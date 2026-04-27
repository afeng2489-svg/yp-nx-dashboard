import { LucideIcon, GitBranch, Play, Terminal, Users, Activity, Settings } from 'lucide-react';
import { cn } from '@/lib/utils';

interface EmptyStateProps {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  variant?: 'default' | 'workflows' | 'executions' | 'sessions' | 'terminal' | 'settings';
  className?: string;
}

// Map variants to icons
const variantIcons: Record<string, LucideIcon> = {
  workflows: GitBranch,
  executions: Play,
  sessions: Users,
  terminal: Terminal,
  settings: Settings,
  default: Activity,
};

export function BetterEmptyState({
  icon,
  title,
  description,
  action,
  variant = 'default',
  className,
}: EmptyStateProps) {
  const Icon = icon || variantIcons[variant] || Activity;

  return (
    <div className={cn('flex flex-col items-center justify-center py-16 px-6', className)}>
      {/* Icon with animated background */}
      <div className="relative mb-6">
        {/* Animated ring */}
        <div className="absolute inset-0 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 animate-pulse-soft" />

        {/* Icon container */}
        <div className="relative w-20 h-20 rounded-2xl bg-gradient-to-br from-indigo-500/10 via-purple-500/10 to-pink-500/10 border border-indigo-500/20 flex items-center justify-center">
          <Icon className="w-10 h-10 text-indigo-500" />
        </div>

        {/* Floating decorative elements */}
        <div
          className="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 opacity-60 animate-bounce"
          style={{ animationDuration: '2s' }}
        />
        <div
          className="absolute -bottom-1 -left-1 w-2 h-2 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 opacity-60 animate-bounce"
          style={{ animationDuration: '2.5s', animationDelay: '0.5s' }}
        />
      </div>

      {/* Text content */}
      <h3 className="text-lg font-semibold text-foreground mb-2">{title}</h3>
      {description && (
        <p className="text-sm text-muted-foreground text-center max-w-sm mb-6">{description}</p>
      )}

      {/* Action button */}
      {action && (
        <button onClick={action.onClick} className="btn-primary">
          {action.label}
        </button>
      )}
    </div>
  );
}
