import { Bug, X, Trash2, Edit2, Layers, Zap, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  Issue,
  IssueStatus,
  issueStatusLabels,
  issueStatusColors,
  issuePriorityLabels,
  issuePriorityColors,
} from '@/stores/issueStore';

interface IssueDetailPanelProps {
  issue: Issue;
  onClose: () => void;
  onTriggerWorkflow: (action: 'plan' | 'queue' | 'execute') => void;
  onDelete: (id: string) => void;
}

const NEXT_ACTIONS: Array<{
  label: string;
  action: 'plan' | 'queue' | 'execute';
  icon: React.ReactNode;
  fromStatus: IssueStatus;
}> = [
  {
    label: '触发 Plan 工作流',
    action: 'plan',
    icon: <Edit2 className="w-4 h-4" />,
    fromStatus: 'discovered',
  },
  {
    label: '触发 Queue 工作流',
    action: 'queue',
    icon: <Layers className="w-4 h-4" />,
    fromStatus: 'planned',
  },
  {
    label: '触发 Execute 工作流',
    action: 'execute',
    icon: <Zap className="w-4 h-4" />,
    fromStatus: 'queued',
  },
];

export function IssueDetailPanel({
  issue,
  onClose,
  onTriggerWorkflow,
  onDelete,
}: IssueDetailPanelProps) {
  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-red-500/5 to-orange-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Bug className="w-5 h-5 text-red-500" />
            Issue 详情
          </h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={cn(
                'px-3 py-1 rounded-full text-xs font-medium border',
                issueStatusColors[issue.status],
              )}
            >
              {issueStatusLabels[issue.status]}
            </span>
            <span
              className={cn(
                'px-3 py-1 rounded-full text-xs font-medium border',
                issuePriorityColors[issue.priority],
              )}
            >
              {issuePriorityLabels[issue.priority]}
            </span>
            {issue.perspectives.map((p) => (
              <span
                key={p}
                className="px-2 py-0.5 rounded-full text-xs border border-indigo-500/30 bg-indigo-500/10 text-indigo-600"
              >
                {p}
              </span>
            ))}
          </div>

          <div>
            <h3 className="font-semibold text-base">{issue.title}</h3>
            <p className="text-sm text-muted-foreground mt-1 whitespace-pre-wrap">
              {issue.description}
            </p>
          </div>

          {issue.solution && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">解决方案</h4>
              <p className="text-sm bg-accent/50 rounded px-3 py-2 whitespace-pre-wrap">
                {issue.solution}
              </p>
            </div>
          )}

          {issue.depends_on.length > 0 && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">依赖</h4>
              <div className="flex flex-wrap gap-1">
                {issue.depends_on.map((id) => (
                  <span key={id} className="text-xs font-mono bg-accent px-2 py-0.5 rounded">
                    {id.slice(0, 8)}...
                  </span>
                ))}
              </div>
            </div>
          )}

          <div className="grid grid-cols-2 gap-4 text-sm text-muted-foreground">
            <div>
              <span className="block text-xs mb-0.5">创建时间</span>
              {new Date(issue.created_at).toLocaleString('zh-CN')}
            </div>
            <div>
              <span className="block text-xs mb-0.5">更新时间</span>
              {new Date(issue.updated_at).toLocaleString('zh-CN')}
            </div>
          </div>

          <div>
            <h4 className="text-xs font-medium text-muted-foreground mb-2">触发工作流</h4>
            <div className="space-y-2">
              {NEXT_ACTIONS.map(({ label, action, icon, fromStatus }) => (
                <button
                  key={action}
                  onClick={() => onTriggerWorkflow(action)}
                  disabled={issue.status !== fromStatus}
                  className={cn(
                    'w-full flex items-center justify-between px-4 py-2.5 rounded-lg border text-sm font-medium transition-all',
                    issue.status === fromStatus
                      ? 'border-indigo-500/30 bg-indigo-500/5 hover:bg-indigo-500/10 text-indigo-600'
                      : 'border-border/30 bg-muted/30 text-muted-foreground cursor-not-allowed opacity-50',
                  )}
                >
                  <span className="flex items-center gap-2">
                    {icon}
                    {label}
                  </span>
                  <ChevronRight className="w-4 h-4" />
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50">
          <button
            onClick={() => onDelete(issue.id)}
            className="btn-secondary text-red-500 hover:bg-red-500/10 flex items-center gap-2"
          >
            <Trash2 className="w-4 h-4" />
            删除
          </button>
        </div>
      </div>
    </div>
  );
}
