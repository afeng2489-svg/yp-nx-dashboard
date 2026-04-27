import { cn } from '@/lib/utils';

// Base skeleton component with shimmer effect
export function Skeleton({ className, shimmer = true }: { className?: string; shimmer?: boolean }) {
  return (
    <div
      className={cn(
        'rounded-lg bg-gradient-to-r from-muted via-muted/50 to-muted',
        shimmer && 'animate-shimmer bg-[length:200%_100%]',
        className,
      )}
    />
  );
}

// Skeleton text lines
export function SkeletonText({ lines = 3, className }: { lines?: number; className?: string }) {
  return (
    <div className={cn('space-y-2', className)}>
      {Array.from({ length: lines }).map((_, i) => (
        <Skeleton key={i} className={cn('h-4', i === lines - 1 ? 'w-3/4' : 'w-full')} />
      ))}
    </div>
  );
}

// Skeleton card
export function SkeletonCard({ className }: { className?: string }) {
  return (
    <div className={cn('bg-card rounded-2xl border border-border/50 p-5', className)}>
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <Skeleton className="w-10 h-10 rounded-xl" />
          <div className="space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-3 w-24" />
          </div>
        </div>
        <Skeleton className="w-16 h-6 rounded-full" />
      </div>
      <SkeletonText lines={2} />
    </div>
  );
}

// Skeleton stat card
export function SkeletonStatCard({ className }: { className?: string }) {
  return (
    <div className={cn('bg-card rounded-2xl border border-border/50 p-5', className)}>
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <Skeleton className="h-3 w-20" />
          <Skeleton className="h-8 w-16" />
        </div>
        <Skeleton className="w-12 h-12 rounded-xl" />
      </div>
    </div>
  );
}

// Skeleton list item
export function SkeletonListItem({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        'flex items-center justify-between p-4 rounded-xl bg-card border border-border/50',
        className,
      )}
    >
      <div className="flex items-center gap-3">
        <Skeleton className="w-10 h-10 rounded-xl" />
        <div className="space-y-2">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-3 w-24" />
        </div>
      </div>
      <Skeleton className="w-16 h-6 rounded-full" />
    </div>
  );
}

// Full page skeleton loader
export function SkeletonPage({ type = 'list' }: { type?: 'dashboard' | 'list' | 'detail' }) {
  if (type === 'dashboard') {
    return (
      <div className="page-container space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <Skeleton className="h-8 w-32" />
            <Skeleton className="h-4 w-48" />
          </div>
          <Skeleton className="h-10 w-28 rounded-lg" />
        </div>

        {/* Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <SkeletonStatCard key={i} />
          ))}
        </div>

        {/* Cards */}
        <div className="space-y-4">
          <SkeletonCard />
          <SkeletonCard />
        </div>
      </div>
    );
  }

  if (type === 'list') {
    return (
      <div className="page-container space-y-6">
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <Skeleton className="h-8 w-32" />
            <Skeleton className="h-4 w-48" />
          </div>
          <Skeleton className="h-10 w-28 rounded-lg" />
        </div>

        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <SkeletonListItem key={i} />
          ))}
        </div>
      </div>
    );
  }

  // Default/detail
  return (
    <div className="page-container space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-4 w-48" />
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SkeletonCard />
        <SkeletonCard />
      </div>
    </div>
  );
}
