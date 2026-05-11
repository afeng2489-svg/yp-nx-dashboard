import type { IssueStatus } from '@/stores/issueStore';
import { ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';

interface IssuePipelineStagesProps {
  statusCounts: Record<IssueStatus, number>;
}

const PIPELINE_STAGES = [
  {
    label: '发现',
    statuses: ['discovered'] as IssueStatus[],
    color: 'from-gray-500 to-slate-500',
    wf: '自动扫描',
  },
  {
    label: '规划',
    statuses: ['planned'] as IssueStatus[],
    color: 'from-blue-500 to-indigo-500',
    wf: 'issue-plan',
  },
  {
    label: '队列',
    statuses: ['queued'] as IssueStatus[],
    color: 'from-yellow-500 to-amber-500',
    wf: 'issue-queue',
  },
  {
    label: '执行',
    statuses: ['executing', 'completed', 'failed'] as IssueStatus[],
    color: 'from-purple-500 to-pink-500',
    wf: 'issue-execute',
  },
] as const;

export function IssuePipelineStages({ statusCounts }: IssuePipelineStagesProps) {
  return (
    <div className="grid grid-cols-4 gap-3">
      {PIPELINE_STAGES.map((stage, i, arr) => {
        const count = stage.statuses.reduce((n, s) => n + statusCounts[s], 0);
        return (
          <div key={stage.label} className="relative">
            <div className={cn('bg-gradient-to-r p-px rounded-xl', stage.color)}>
              <div className="bg-card rounded-xl p-3">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-semibold text-muted-foreground">Step {i + 1}</span>
                  <span
                    className={cn(
                      'text-lg font-bold bg-gradient-to-r bg-clip-text text-transparent',
                      stage.color,
                    )}
                  >
                    {count}
                  </span>
                </div>
                <p className="text-sm font-medium">{stage.label}</p>
                <p className="text-xs text-muted-foreground">{stage.wf}</p>
              </div>
            </div>
            {i < arr.length - 1 && (
              <div className="absolute -right-1.5 top-1/2 -translate-y-1/2 z-10 text-muted-foreground">
                <ChevronRight className="w-3 h-3" />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
