import { useEffect } from 'react';
import { GitBranch, Loader2 } from 'lucide-react';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useSnapshotStore } from '@/stores/snapshotStore';
import { cn } from '@/lib/utils';

export interface WorkspacePipelineSummaryProps {
  workspaceId?: string;
  className?: string;
}

/** Phase D — Context Panel 只读 Pipeline 摘要 */
export function WorkspacePipelineSummary({ workspaceId, className }: WorkspacePipelineSummaryProps) {
  const { progress, fetchProgress, progressLoading } = useSnapshotStore();

  useEffect(() => {
    if (workspaceId) void fetchProgress(workspaceId);
  }, [workspaceId, fetchProgress]);

  if (!workspaceId || progressLoading) {
    if (progressLoading) {
      return (
        <div className={cn('flex items-center gap-2 text-xs text-muted-foreground', className)}>
          <Loader2 className="h-3 w-3 animate-spin" />
          Pipeline…
        </div>
      );
    }
    return null;
  }

  if (!progress?.pipeline_id) return null;

  return (
    <div
      className={cn(
        'rounded-lg border border-indigo-500/20 bg-indigo-500/5 px-3 py-2 text-xs space-y-1',
        className,
      )}
      data-testid="workspace-pipeline-summary"
    >
      <div className="flex items-center gap-1.5 font-medium text-indigo-700 dark:text-indigo-300">
        <GitBranch className="h-3.5 w-3.5" />
        长周期 Pipeline
      </div>
      <div className="text-muted-foreground">
        阶段 {progress.overall_phase} · {progress.overall_pct}%
      </div>
      <div className="h-1.5 rounded-full bg-secondary overflow-hidden">
        <div
          className="h-full bg-indigo-500 transition-all"
          style={{ width: `${progress.overall_pct}%` }}
        />
      </div>
    </div>
  );
}
