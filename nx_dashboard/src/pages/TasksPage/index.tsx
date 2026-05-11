import { useEffect, useState } from 'react';
import { useTasksQuery } from '@/hooks/useReactQuery';
import {
  useTaskStore,
  TaskStatus,
  TaskPriority,
  ExecutionMode,
  taskStatusLabels,
} from '@/stores/taskStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { List, Bug, RefreshCw, Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { CreateTaskModal } from './CreateTaskModal';
import { TaskDetailPanel } from './TaskDetailPanel';
import { TasksTab } from './TasksTab';
import { IssuesTab } from './IssuesTab';

type PageTab = 'tasks' | 'issues';

export function TasksPage() {
  const { error, createTask, cancelTask, updateTask } = useTaskStore();
  const [selectedTask, setSelectedTask] = useState<
    ReturnType<typeof useTaskStore.getState>['tasks'][0] | null
  >(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');
  const [activeTab, setActiveTab] = useState<PageTab>('tasks');
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const { tasks, stats, loading, refetch } = useTasksQuery();

  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => {
      unsubscribe();
    };
  }, [refetch]);

  const handleCreateTask = async (
    name: string,
    description: string,
    priority: TaskPriority,
    executionMode: ExecutionMode,
  ) => {
    await createTask({ name, description, priority, execution_mode: executionMode });
    refetch();
  };

  const handleCancelTask = async (id: string) => {
    showConfirm(
      '取消任务',
      '确定要取消该任务吗？',
      async () => {
        await cancelTask(id);
        setSelectedTask(null);
        refetch();
      },
      'danger',
    );
  };

  const handleUpdateStatus = async (id: string, status: TaskStatus) => {
    const label = taskStatusLabels[status];
    showConfirm(
      '更新状态',
      `确定要将任务状态改为「${label}」吗？`,
      async () => {
        await updateTask(id, { status });
        setSelectedTask(null);
        refetch();
      },
      'warning',
    );
  };

  return (
    <div className="page-container space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              {activeTab === 'tasks' ? '后台任务' : 'Issue 管理'}
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">
            {activeTab === 'tasks'
              ? '管理和监控后台定时任务'
              : 'Discover → Plan → Queue → Execute 全闭环'}
          </p>
        </div>
        {activeTab === 'tasks' && (
          <div className="flex items-center gap-2">
            <button onClick={() => refetch()} className="btn-secondary flex items-center gap-2">
              <RefreshCw className="w-4 h-4" />
              刷新
            </button>
            <button
              onClick={() => setShowCreateModal(true)}
              className="btn-primary flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              创建任务
            </button>
          </div>
        )}
      </div>

      <div className="flex items-center gap-1 bg-accent/50 rounded-xl p-1 w-fit">
        <button
          onClick={() => setActiveTab('tasks')}
          className={cn(
            'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
            activeTab === 'tasks'
              ? 'bg-card shadow-sm text-foreground'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          <List className="w-4 h-4" />
          后台任务
        </button>
        <button
          onClick={() => setActiveTab('issues')}
          className={cn(
            'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
            activeTab === 'issues'
              ? 'bg-card shadow-sm text-foreground'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          <Bug className="w-4 h-4" />
          Issue 管理
        </button>
      </div>

      {activeTab === 'issues' ? (
        <IssuesTab />
      ) : (
        <TasksTab
          tasks={tasks}
          stats={stats}
          loading={loading}
          error={error}
          filter={filter}
          onFilterChange={setFilter}
          onTaskClick={setSelectedTask}
          onCreateClick={() => setShowCreateModal(true)}
        />
      )}

      {showCreateModal && (
        <CreateTaskModal
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)}
          onSubmit={handleCreateTask}
        />
      )}

      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          onClose={() => setSelectedTask(null)}
          onCancel={handleCancelTask}
          onUpdateStatus={handleUpdateStatus}
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
