import { useEffect, useState } from 'react';
import {
  useIssueStore,
  IssueStatus,
  IssuePriority,
  issueStatusLabels,
  issueStatusColors,
  issuePriorityColors,
  issuePriorityLabels,
  Issue,
} from '@/stores/issueStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { RefreshCw, Plus, Search, AlertCircle, Bug } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showSuccess } from '@/lib/toast';
import { CreateIssueModal } from './CreateIssueModal';
import { IssueDetailPanel } from './IssueDetailPanel';
import { IssuePipelineStages } from './IssuePipelineStages';
import { triggerIssueWorkflow, triggerDiscoverWorkflow } from './issueWorkflowUtils';

const FILTER_STATUSES = [
  'discovered',
  'planned',
  'queued',
  'executing',
  'completed',
  'failed',
] as IssueStatus[];

export function IssuesTab() {
  const { issues, loading, error, fetchIssues, createIssue, deleteIssue } = useIssueStore();
  const { workflows, fetchWorkflows } = useWorkflowStore();
  const [statusFilter, setStatusFilter] = useState<IssueStatus | 'all'>('all');
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  useEffect(() => {
    fetchIssues();
    fetchWorkflows();
  }, [fetchIssues, fetchWorkflows]);

  const filtered =
    statusFilter === 'all' ? issues : issues.filter((i) => i.status === statusFilter);

  const handleCreate = async (
    title: string,
    description: string,
    priority: IssuePriority,
    perspectives: string[],
  ) => {
    const issue = await createIssue({ title, description, priority, perspectives });
    if (issue) showSuccess('Issue 创建成功');
  };

  const handleDelete = (id: string) => {
    showConfirm(
      '删除 Issue',
      '确定要删除该 Issue 吗？此操作不可撤销。',
      async () => {
        const ok = await deleteIssue(id);
        if (ok) {
          setSelectedIssue(null);
          showSuccess('已删除');
        }
      },
      'danger',
    );
  };

  const statusCounts = FILTER_STATUSES.reduce(
    (acc, s) => {
      acc[s] = issues.filter((i) => i.status === s).length;
      return acc;
    },
    {} as Record<IssueStatus, number>,
  );

  return (
    <div className="space-y-5">
      <IssuePipelineStages statusCounts={statusCounts} />

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 flex-wrap">
          <button
            onClick={() => setStatusFilter('all')}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-colors border',
              statusFilter === 'all'
                ? 'bg-indigo-500/10 text-indigo-600 border-indigo-500/30'
                : 'border-border/50 bg-card hover:bg-accent',
            )}
          >
            全部 ({issues.length})
          </button>
          {FILTER_STATUSES.map((s) => (
            <button
              key={s}
              onClick={() => setStatusFilter(s)}
              className={cn(
                'px-3 py-1 rounded-full text-xs font-medium transition-colors border',
                statusFilter === s
                  ? issueStatusColors[s]
                  : 'border-border/50 bg-card hover:bg-accent',
              )}
            >
              {issueStatusLabels[s]} ({statusCounts[s]})
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => fetchIssues()}
            className="btn-secondary flex items-center gap-1.5 text-sm py-1.5 px-3"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            刷新
          </button>
          <button
            onClick={() => triggerDiscoverWorkflow(workflows)}
            className="btn-secondary flex items-center gap-1.5 text-sm py-1.5 px-3 text-orange-600 hover:bg-orange-500/10"
          >
            <Search className="w-3.5 h-3.5" />
            发现
          </button>
          <button
            onClick={() => setShowCreate(true)}
            className="btn-primary flex items-center gap-1.5 text-sm py-1.5 px-3"
          >
            <Plus className="w-3.5 h-3.5" />
            新建
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-3 flex items-center gap-2 text-sm text-red-500">
          <AlertCircle className="w-4 h-4 shrink-0" />
          {error}
        </div>
      )}

      {loading && issues.length === 0 && (
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-16 bg-muted rounded-xl" />
          ))}
        </div>
      )}

      {filtered.length === 0 && !loading ? (
        <div className="text-center py-12 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <Bug className="w-10 h-10 mx-auto mb-3 text-muted-foreground/40" />
          <p className="text-muted-foreground text-sm">暂无 Issue</p>
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((issue) => (
            <div
              key={issue.id}
              onClick={() => setSelectedIssue(issue)}
              className="bg-card rounded-xl border border-border/50 p-4 hover:border-red-500/20 hover:shadow-sm transition-all cursor-pointer"
            >
              <div className="flex items-center justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1 flex-wrap">
                    <span
                      className={cn(
                        'px-2 py-0.5 rounded-full text-xs font-medium border',
                        issueStatusColors[issue.status],
                      )}
                    >
                      {issueStatusLabels[issue.status]}
                    </span>
                    <span
                      className={cn(
                        'px-2 py-0.5 rounded-full text-xs font-medium border',
                        issuePriorityColors[issue.priority],
                      )}
                    >
                      {issuePriorityLabels[issue.priority]}
                    </span>
                    {issue.perspectives.slice(0, 2).map((p) => (
                      <span
                        key={p}
                        className="px-1.5 py-0.5 rounded text-xs border border-indigo-500/20 bg-indigo-500/5 text-indigo-500"
                      >
                        {p}
                      </span>
                    ))}
                  </div>
                  <p className="font-medium text-sm truncate">{issue.title}</p>
                  <p className="text-xs text-muted-foreground truncate mt-0.5">
                    {issue.description}
                  </p>
                </div>
                <div className="text-xs text-muted-foreground shrink-0">
                  {new Date(issue.updated_at).toLocaleString('zh-CN', {
                    month: 'numeric',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {showCreate && (
        <CreateIssueModal
          isOpen={showCreate}
          onClose={() => setShowCreate(false)}
          onSubmit={handleCreate}
        />
      )}
      {selectedIssue && (
        <IssueDetailPanel
          issue={selectedIssue}
          onClose={() => setSelectedIssue(null)}
          onTriggerWorkflow={(action) =>
            triggerIssueWorkflow(selectedIssue, action, workflows, () => setSelectedIssue(null))
          }
          onDelete={handleDelete}
        />
      )}
      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={() => {
          confirmState.onConfirm();
          hideConfirm();
        }}
        onCancel={hideConfirm}
        variant={confirmState.variant || 'danger'}
      />
    </div>
  );
}
