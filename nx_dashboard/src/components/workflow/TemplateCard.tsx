import { GitBranch, Users, Play } from 'lucide-react';
import type { TemplateSummary } from '@/stores/templateStore';
import { cn } from '@/lib/utils';

interface TemplateCardProps {
  template: TemplateSummary;
  onLaunch: () => void;
  variant?: 'default' | 'compact';
}

const categoryColors: Record<string, string> = {
  planning: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  development: 'bg-green-500/10 text-green-600 border-green-500/20',
  analysis: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  security: 'bg-red-500/10 text-red-600 border-red-500/20',
  testing: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  research: 'bg-cyan-500/10 text-cyan-600 border-cyan-500/20',
  writing: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
};

export function TemplateCard({ template, onLaunch, variant = 'default' }: TemplateCardProps) {
  const categoryColorClass = categoryColors[template.category] || categoryColors.planning;

  if (variant === 'compact') {
    return (
      <div
        className="
          group p-4 rounded-lg border border-border bg-card
          hover:border-primary/50 hover:bg-accent/30
          transition-all cursor-pointer
        "
        onClick={onLaunch}
      >
        <div className="flex items-center gap-3">
          <div className={cn('p-2 rounded-lg border', categoryColorClass)}>
            <Play className="w-4 h-4" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-medium text-sm truncate">{template.name}</h3>
            <p className="text-xs text-muted-foreground capitalize">{template.category}</p>
          </div>
          <Play className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors" />
        </div>
      </div>
    );
  }

  return (
    <div className="p-5 rounded-xl border border-border bg-card hover:border-primary/50 hover:shadow-md transition-all">
      {/* Header */}
      <div className="flex items-start gap-3 mb-4">
        <div className={cn('p-2.5 rounded-lg border', categoryColorClass)}>
          <Play className="w-5 h-5" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-base truncate">{template.name}</h3>
          <span
            className={cn(
              'inline-block px-2 py-0.5 text-xs rounded border capitalize mt-1',
              categoryColorClass,
            )}
          >
            {template.category}
          </span>
        </div>
      </div>

      {/* Description */}
      <p className="text-sm text-muted-foreground line-clamp-2 mb-4">{template.description}</p>

      {/* Stats */}
      <div className="flex items-center gap-4 mb-4 text-xs text-muted-foreground">
        <div className="flex items-center gap-1.5">
          <GitBranch className="w-3.5 h-3.5" />
          <span>{template.stage_count} 阶段</span>
        </div>
        <div className="flex items-center gap-1.5">
          <Users className="w-3.5 h-3.5" />
          <span>{template.agent_count} 智能体</span>
        </div>
      </div>

      {/* Launch button */}
      <div className="pt-3 border-t border-border">
        <button
          onClick={onLaunch}
          className="
            w-full py-2 text-sm rounded-md bg-primary text-primary-foreground
            hover:opacity-90 transition-opacity
            flex items-center justify-center gap-1.5
          "
        >
          <Play className="w-3.5 h-3.5" />
          启动
        </button>
      </div>
    </div>
  );
}

export function TemplateCardSkeleton() {
  return (
    <div className="p-5 rounded-xl border border-border bg-card animate-pulse">
      <div className="flex items-start gap-3 mb-4">
        <div className="w-10 h-10 rounded-lg bg-muted" />
        <div className="flex-1">
          <div className="h-5 w-32 bg-muted rounded mb-2" />
          <div className="h-4 w-16 bg-muted rounded" />
        </div>
      </div>
      <div className="space-y-2 mb-4">
        <div className="h-4 bg-muted rounded w-full" />
        <div className="h-4 bg-muted rounded w-3/4" />
      </div>
      <div className="flex items-center gap-4 mb-4">
        <div className="h-4 w-20 bg-muted rounded" />
        <div className="h-4 w-20 bg-muted rounded" />
      </div>
      <div className="flex items-center gap-2 pt-3 border-t border-border">
        <div className="flex-1 h-8 bg-muted rounded-md" />
        <div className="flex-1 h-8 bg-muted rounded-md" />
      </div>
    </div>
  );
}
