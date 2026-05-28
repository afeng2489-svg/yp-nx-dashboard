import { useEffect, useState, useCallback, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { useExecutionsQuery } from '@/hooks/useReactQuery';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { Loader2, Trash2 } from 'lucide-react';
import { showSuccess, showError } from '@/lib/toast';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { ErrorState } from '@/components/ui/ErrorState';
import { ExecutionDetailModal } from './ExecutionDetailModal';
import { ExecutionCard } from './ExecutionCard';
import { ExecutionFiltersBar, useExecutionFilters } from './ExecutionFilters';
import { WorkflowOperationsGuide } from './WorkflowOperationsGuide';
import { Pagination } from '@/components/ui/Pagination';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useIsStudioDark } from '@/components/layout/ShellThemeContext';
import { cn } from '@/lib/utils';
import { PageHeader } from '@/components/ui/PageHeader';

const PAGE_SIZE = 6;
const EMPTY_ACTIONS = [
  { label: '前往工作流', onClick: () => (window.location.href = '/assets?tab=workflows') },
  { label: '返回工厂台', onClick: () => (window.location.href = '/factory'), variant: 'secondary' as const },
];

export function ExecutionsPage({
  embedded = false,
  detailMode = 'modal',
  showFilters = !embedded,
  runsOnly = false,
}: {
  embedded?: boolean;
  detailMode?: 'modal' | 'context-only';
  showFilters?: boolean;
  /** 工厂「运行中」Tab：仅展示 running + paused */
  runsOnly?: boolean;
}) {
  const location = useLocation();
  const { cancelExecution, deleteExecutions, connectWebSocket, getExecution } = useExecutionStore();
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);
  const [fullExecution, setFullExecution] = useState<Execution | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [cancellingIds, setCancellingIds] = useState<Set<string>>(new Set());
  const [page, setPage] = useState(1);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();
  const { executions, loading, refetch } = useExecutionsQuery();
  const sourceExecutions = useMemo(
    () =>
      runsOnly
        ? executions.filter((e) => e.status === 'running' || e.status === 'paused')
        : executions,
    [executions, runsOnly],
  );
  const { filters, setFilters, filtered, teams, projects, workflows, reset } =
    useExecutionFilters(sourceExecutions);
  const selectContextExecution = useContextPanelStore((s) => s.selectExecution);
  const isStudio = useIsStudioDark();

  const liveSelectedExecution = selectedExecutionId
    ? (executions.find((e) => e.id === selectedExecutionId) ?? null)
    : null;

  // 初始化：加载工作流 + 监听 workspace 变更
  useEffect(() => {
    fetchWorkflows();
    void useTeamStore.getState().fetchTeams();
    void useProjectStore.getState().fetchProjects();
  }, [fetchWorkflows]);
  useEffect(() => {
    const unsub = onWorkspaceChange(() => {
      refetch();
    });
    return () => {
      unsub();
    };
  }, [refetch]);

  // 从路由 state 打开指定执行
  useEffect(() => {
    const state = location.state as { openExecutionId?: string } | null;
    if (state?.openExecutionId) setSelectedExecutionId(state.openExecutionId);
  }, [location.state]);

  // 拉取完整执行详情（列表接口不含 stage_results）
  useEffect(() => {
    if (!selectedExecutionId) {
      setFullExecution(null);
      return;
    }
    let cancelled = false;
    getExecution(selectedExecutionId).then((full) => {
      if (!cancelled) setFullExecution(full);
    });
    return () => {
      cancelled = true;
    };
  }, [selectedExecutionId, getExecution]);

  const runningWsKey = useMemo(
    () =>
      executions
        .filter((e) => e.status === 'running')
        .map((e) => e.id)
        .sort()
        .join(','),
    [executions],
  );

  // 为 running 执行建立 WebSocket 连接
  useEffect(() => {
    if (liveSelectedExecution?.status === 'running') connectWebSocket(liveSelectedExecution.id);
  }, [liveSelectedExecution?.id, liveSelectedExecution?.status, connectWebSocket]);
  useEffect(() => {
    if (!runningWsKey) return;
    for (const id of runningWsKey.split(',')) {
      connectWebSocket(id);
    }
  }, [runningWsKey, connectWebSocket]);

  const handleCancel = useCallback(
    async (id: string) => {
      setCancellingIds((prev) => new Set(prev).add(id));
      try {
        const result = await cancelExecution(id);
        if (result.ok) {
          showSuccess('已取消运行');
        } else {
          showError(result.error ?? '取消失败');
        }
      } finally {
        setCancellingIds((prev) => {
          const next = new Set(prev);
          next.delete(id);
          return next;
        });
      }
    },
    [cancelExecution],
  );

  const toggleSelect = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const toggleSelectAll = useCallback(() => {
    setSelectedIds((prev) =>
      prev.size === executions.length ? new Set() : new Set(executions.map((e) => e.id)),
    );
  }, [executions]);

  const handleDeleteSelected = useCallback(() => {
    const ids = [...selectedIds];
    showConfirm(
      '删除执行记录',
      `确定删除选中的 ${ids.length} 条执行记录？此操作不可撤销。`,
      async () => {
        try {
          await deleteExecutions(ids);
          setSelectedIds(new Set());
          setPage(1);
          showSuccess(`已删除 ${ids.length} 条记录`);
        } catch {
          showError('删除失败，请重试');
        }
      },
      'danger',
    );
  }, [selectedIds, deleteExecutions, showConfirm]);

  const totalPages = Math.ceil(filtered.length / PAGE_SIZE);
  const pagedExecutions = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  if (loading) return <LoadingState />;

  return (
    <div className={embedded ? (isStudio ? 'space-y-3' : 'space-y-4') : 'page-container space-y-6'}>
      {!embedded && (
        <PageHeader
          title="执行记录"
          description="查看所有工作流执行历史"
          badges={
            <span className="px-3 py-1.5 rounded-full bg-primary/10 text-primary text-sm font-medium border border-primary/20">
              {executions.length} 条记录
            </span>
          }
          actions={
            <div className="flex items-center gap-3">
              {selectedIds.size > 0 && (
                <button
                  onClick={handleDeleteSelected}
                  className="flex items-center gap-2 px-4 py-2 rounded-xl bg-destructive text-destructive-foreground text-sm font-medium hover:opacity-90 transition-opacity"
                >
                  <Trash2 className="w-4 h-4" /> 删除 ({selectedIds.size})
                </button>
              )}
              {executions.length > 0 && (
                <button
                  onClick={toggleSelectAll}
                  className="px-3 py-1.5 rounded-xl text-sm border border-border/50 hover:bg-accent transition-colors"
                >
                  {selectedIds.size === executions.length ? '取消全选' : '全选'}
                </button>
              )}
            </div>
          }
        />
      )}

      {!embedded && <WorkflowOperationsGuide />}

      {showFilters && (
        <ExecutionFiltersBar
          filters={filters}
          onChange={setFilters}
          onReset={reset}
          teams={teams}
          projects={projects}
          workflows={workflows}
        />
      )}

      {sourceExecutions.length === 0 ? (
        <ErrorState
          variant="empty"
          title={runsOnly ? '暂无运行中的 Run' : '暂无执行记录'}
          message={
            runsOnly
              ? '从工厂控制台启动工作流后，进行中的 Run 会出现在这里'
              : '从工作流列表或首页选择一个工作流开始执行'
          }
          actions={EMPTY_ACTIONS}
        />
      ) : (
        <>
          <div className={cn('grid gap-3', isStudio ? 'md:grid-cols-2' : 'gap-4 md:grid-cols-2 lg:grid-cols-2 stagger-children')}>
            {pagedExecutions.map((execution) => (
              <ExecutionCard
                key={execution.id}
                execution={execution}
                onClick={() => {
                  selectContextExecution(execution.id);
                  if (detailMode !== 'context-only') {
                    setSelectedExecutionId(execution.id);
                  }
                }}
                onCancel={handleCancel}
                isCancelling={cancellingIds.has(execution.id)}
                selected={selectedIds.has(execution.id)}
                onSelect={toggleSelect}
              />
            ))}
          </div>
          <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
        </>
      )}

      {(fullExecution ?? liveSelectedExecution) && detailMode === 'modal' && (
        <ExecutionDetailModal
          execution={fullExecution ?? liveSelectedExecution!}
          onClose={() => setSelectedExecutionId(null)}
          onCancel={handleCancel}
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

function LoadingState() {
  return (
    <div className="page-container">
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-muted/60 flex items-center justify-center animate-pulse">
            <Loader2 className="w-8 h-8 text-primary animate-spin" />
          </div>
          <p className="text-muted-foreground">加载中...</p>
        </div>
      </div>
    </div>
  );
}
