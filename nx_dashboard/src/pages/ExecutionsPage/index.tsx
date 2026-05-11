import { useEffect, useState, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { useExecutionsQuery } from '@/hooks/useReactQuery';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { Loader2, Trash2 } from 'lucide-react';
import { showSuccess, showError } from '@/lib/toast';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { ErrorState } from '@/components/ui/ErrorState';
import { ExecutionDetailModal } from './ExecutionDetailModal';
import { ExecutionCard } from './ExecutionCard';
import { WorkflowOperationsGuide } from './WorkflowOperationsGuide';
import { Pagination } from './Pagination';

const PAGE_SIZE = 10;
const EMPTY_ACTIONS = [
  { label: '前往工作流', onClick: () => (window.location.href = '/workflows') },
  { label: '返回首页', onClick: () => (window.location.href = '/'), variant: 'secondary' as const },
];

export function ExecutionsPage() {
  const location = useLocation();
  const { cancelExecution, deleteExecutions, connectWebSocket, getExecution } = useExecutionStore();
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);
  const [fullExecution, setFullExecution] = useState<Execution | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [page, setPage] = useState(1);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();
  const { executions, loading, refetch } = useExecutionsQuery();

  const liveSelectedExecution = selectedExecutionId
    ? (executions.find((e) => e.id === selectedExecutionId) ?? null)
    : null;

  // 初始化：加载工作流 + 监听 workspace 变更
  useEffect(() => {
    fetchWorkflows();
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

  // 为 running 执行建立 WebSocket 连接
  useEffect(() => {
    if (liveSelectedExecution?.status === 'running') connectWebSocket(liveSelectedExecution.id);
  }, [liveSelectedExecution?.id, liveSelectedExecution?.status, connectWebSocket]);
  useEffect(() => {
    executions.filter((e) => e.status === 'running').forEach((e) => connectWebSocket(e.id));
  }, [executions, connectWebSocket]);

  const handleCancel = useCallback(
    (id: string) => {
      cancelExecution(id);
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

  const totalPages = Math.ceil(executions.length / PAGE_SIZE);
  const pagedExecutions = executions.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  if (loading) return <LoadingState />;

  return (
    <div className="page-container space-y-6">
      <PageHeader
        recordCount={executions.length}
        selectedCount={selectedIds.size}
        allSelected={selectedIds.size === executions.length && executions.length > 0}
        onDeleteSelected={handleDeleteSelected}
        onToggleSelectAll={toggleSelectAll}
      />

      <WorkflowOperationsGuide />

      {executions.length === 0 ? (
        <ErrorState
          variant="empty"
          title="暂无执行记录"
          message="从工作流列表或首页选择一个工作流开始执行"
          actions={EMPTY_ACTIONS}
        />
      ) : (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-2 stagger-children">
            {pagedExecutions.map((execution) => (
              <ExecutionCard
                key={execution.id}
                execution={execution}
                onClick={() => setSelectedExecutionId(execution.id)}
                onCancel={handleCancel}
                selected={selectedIds.has(execution.id)}
                onSelect={toggleSelect}
              />
            ))}
          </div>
          <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
        </>
      )}

      {(fullExecution ?? liveSelectedExecution) && (
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
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center animate-pulse">
            <Loader2 className="w-8 h-8 text-indigo-500 animate-spin" />
          </div>
          <p className="text-muted-foreground">加载中...</p>
        </div>
      </div>
    </div>
  );
}

interface PageHeaderProps {
  recordCount: number;
  selectedCount: number;
  allSelected: boolean;
  onDeleteSelected: () => void;
  onToggleSelectAll: () => void;
}

function PageHeader({
  recordCount,
  selectedCount,
  allSelected,
  onDeleteSelected,
  onToggleSelectAll,
}: PageHeaderProps) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">
          <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
            执行记录
          </span>
        </h1>
        <p className="text-muted-foreground mt-1">查看所有工作流执行历史</p>
      </div>
      <div className="flex items-center gap-3">
        {selectedCount > 0 && (
          <button
            onClick={onDeleteSelected}
            className="flex items-center gap-2 px-4 py-2 rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white text-sm font-medium shadow-lg shadow-red-500/25 hover:shadow-red-500/40 hover:-translate-y-0.5 transition-all"
          >
            <Trash2 className="w-4 h-4" /> 删除 ({selectedCount})
          </button>
        )}
        {recordCount > 0 && (
          <button
            onClick={onToggleSelectAll}
            className="px-3 py-1.5 rounded-xl text-sm border border-border/50 hover:bg-accent transition-colors"
          >
            {allSelected ? '取消全选' : '全选'}
          </button>
        )}
        <span className="px-3 py-1.5 rounded-full bg-indigo-500/10 text-indigo-600 text-sm font-medium border border-indigo-500/20">
          {recordCount} 条记录
        </span>
      </div>
    </div>
  );
}
