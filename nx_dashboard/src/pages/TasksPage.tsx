import { useEffect, useState } from 'react';
import { useTasksQuery } from '@/hooks/useReactQuery';
import { useTaskStore, TaskType, TaskStatus, taskTypeLabels, taskStatusLabels, taskStatusColors, taskTypeColors } from '@/stores/taskStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { Clock, Play, Pause, Trash2, Plus, X, RefreshCw, CheckCircle, AlertCircle, Timer, List } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';

// Task creation modal
interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (taskType: TaskType, payload: Record<string, unknown>, delaySeconds?: number, maxRetries?: number) => void;
}

function CreateTaskModal({ isOpen, onClose, onSubmit }: CreateTaskModalProps) {
  const [taskType, setTaskType] = useState<TaskType>('workflow_execution');
  const [payload, setPayload] = useState('{}');
  const [delaySeconds, setDelaySeconds] = useState<string>('');
  const [maxRetries, setMaxRetries] = useState<string>('3');

  if (!isOpen) return null;

  const handleSubmit = () => {
    let parsedPayload: Record<string, unknown>;
    try {
      parsedPayload = JSON.parse(payload);
    } catch {
      showError('Invalid JSON payload');
      return;
    }
    onSubmit(
      taskType,
      parsedPayload,
      delaySeconds ? parseInt(delaySeconds, 10) : undefined,
      maxRetries ? parseInt(maxRetries, 10) : undefined
    );
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-gradient-to-r from-black/40 to-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-2xl shadow-2xl border border-border/50 p-6 animate-in fade-in zoom-in duration-200">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <Plus className="w-5 h-5 text-indigo-500" />
            Create Task
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-accent rounded-lg transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">Task Type</label>
            <select
              value={taskType}
              onChange={(e) => setTaskType(e.target.value as TaskType)}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              <option value="workflow_execution">Workflow Execution</option>
              <option value="code_review">Code Review</option>
              <option value="security_audit">Security Audit</option>
              <option value="cleanup">Cleanup</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Payload (JSON)</label>
            <textarea
              value={payload}
              onChange={(e) => setPayload(e.target.value)}
              rows={4}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 font-mono text-sm"
              placeholder='{"key": "value"}'
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-2">Delay (seconds)</label>
              <input
                type="number"
                value={delaySeconds}
                onChange={(e) => setDelaySeconds(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500"
                placeholder="0 (immediate)"
                min="0"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-2">Max Retries</label>
              <input
                type="number"
                value={maxRetries}
                onChange={(e) => setMaxRetries(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500"
                placeholder="3"
                min="0"
              />
            </div>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 mt-6">
          <button onClick={onClose} className="btn-secondary">
            Cancel
          </button>
          <button onClick={handleSubmit} className="btn-primary">
            Create Task
          </button>
        </div>
      </div>
    </div>
  );
}

// Task detail panel
interface TaskDetailPanelProps {
  task: ReturnType<typeof useTaskStore.getState>['tasks'][0];
  onClose: () => void;
  onCancel: (id: string) => void;
}

function TaskDetailPanel({ task, onClose, onCancel }: TaskDetailPanelProps) {
  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-gradient-to-r from-black/20 to-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <List className="w-5 h-5 text-indigo-500" />
            Task Details
          </h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          <div className="space-y-2">
            <span className={cn('px-3 py-1 rounded-full text-xs font-medium border', taskTypeColors[task.task_type])}>
              {taskTypeLabels[task.task_type]}
            </span>
            <span className={cn('px-3 py-1 rounded-full text-xs font-medium border ml-2', taskStatusColors[task.status])}>
              {taskStatusLabels[task.status]}
            </span>
          </div>

          <div className="space-y-3">
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">Task ID</h4>
              <p className="text-sm font-mono bg-accent/50 rounded px-3 py-2">{task.id}</p>
            </div>

            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">Payload</h4>
              <pre className="text-sm font-mono bg-accent/50 rounded px-3 py-2 overflow-x-auto">
                {JSON.stringify(task.payload, null, 2)}
              </pre>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">Scheduled At</h4>
                <p className="text-sm">{new Date(task.scheduled_at).toLocaleString()}</p>
              </div>
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">Execute At</h4>
                <p className="text-sm">{new Date(task.execute_at).toLocaleString()}</p>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">Retry Count</h4>
                <p className="text-sm">{task.retry_count} / {task.max_retries}</p>
              </div>
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">Last Updated</h4>
                <p className="text-sm">{new Date(task.updated_at).toLocaleString()}</p>
              </div>
            </div>

            {task.error_message && (
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">Error Message</h4>
                <p className="text-sm text-red-500 bg-red-500/10 rounded px-3 py-2">{task.error_message}</p>
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          {task.status === 'pending' && (
            <button
              onClick={() => onCancel(task.id)}
              className="btn-secondary text-red-500 hover:bg-red-500/10"
            >
              <Trash2 className="w-4 h-4" />
              Cancel Task
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export function TasksPage() {
  const { error, createTask, cancelTask } = useTaskStore();
  const [selectedTask, setSelectedTask] = useState<ReturnType<typeof useTaskStore.getState>['tasks'][0] | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  // Use React Query for fetching with auto-polling
  const { tasks, stats, loading, refetch } = useTasksQuery();

  // Listen for workspace changes
  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => { unsubscribe(); };
  }, [refetch]);

  const filteredTasks = filter === 'all' ? tasks : tasks.filter((t) => t.status === filter);

  const handleCreateTask = async (taskType: TaskType, payload: Record<string, unknown>, delaySeconds?: number, maxRetries?: number) => {
    await createTask({
      task_type: taskType,
      payload,
      delay_seconds: delaySeconds,
      max_retries: maxRetries,
    });
  };

  const handleCancelTask = async (id: string) => {
    showConfirm(
      'Cancel Task',
      'Are you sure you want to cancel this task?',
      async () => {
        await cancelTask(id);
        setSelectedTask(null);
      },
      'danger'
    );
  };

  return (
    <div className="page-container space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              Tasks
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">Manage scheduled background tasks</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => refetch()} className="btn-secondary">
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            <Plus className="w-4 h-4" />
            Create Task
          </button>
        </div>
      </div>

      {/* Stats Cards */}
      {stats && (
        <div className="grid grid-cols-4 gap-4">
          <div className="bg-card rounded-xl border border-border/50 p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-yellow-500/10">
                <Timer className="w-5 h-5 text-yellow-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Pending</p>
                <p className="text-2xl font-bold">{stats.pending}</p>
              </div>
            </div>
          </div>
          <div className="bg-card rounded-xl border border-border/50 p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-blue-500/10">
                <Play className="w-5 h-5 text-blue-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Running</p>
                <p className="text-2xl font-bold">{stats.running}</p>
              </div>
            </div>
          </div>
          <div className="bg-card rounded-xl border border-border/50 p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-green-500/10">
                <CheckCircle className="w-5 h-5 text-green-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Completed</p>
                <p className="text-2xl font-bold">{stats.completed}</p>
              </div>
            </div>
          </div>
          <div className="bg-card rounded-xl border border-border/50 p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-red-500/10">
                <AlertCircle className="w-5 h-5 text-red-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Failed</p>
                <p className="text-2xl font-bold">{stats.failed}</p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Filter */}
      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground">Filter:</span>
        <button
          onClick={() => setFilter('all')}
          className={cn(
            'px-3 py-1 rounded-full text-xs font-medium transition-colors',
            filter === 'all' ? 'bg-indigo-500/10 text-indigo-600' : 'bg-accent hover:bg-accent/80'
          )}
        >
          All
        </button>
        {(['pending', 'running', 'completed', 'failed', 'cancelled'] as TaskStatus[]).map((status) => (
          <button
            key={status}
            onClick={() => setFilter(status)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-colors',
              filter === status ? taskStatusColors[status] : 'bg-accent hover:bg-accent/80'
            )}
          >
            {taskStatusLabels[status]}
          </button>
        ))}
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-4 flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-red-500" />
          <p className="text-sm text-red-500">{error}</p>
        </div>
      )}

      {/* Loading */}
      {loading && tasks.length === 0 && (
        <div className="animate-pulse space-y-4">
          <div className="h-20 bg-muted rounded-xl" />
          <div className="h-20 bg-muted rounded-xl" />
        </div>
      )}

      {/* Task List */}
      {filteredTasks.length === 0 && !loading ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <List className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">No tasks found</h3>
          <p className="text-muted-foreground mb-4">Create your first task to get started</p>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            <Plus className="w-4 h-4" />
            Create Task
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          {filteredTasks.map((task) => (
            <div
              key={task.id}
              className="bg-card rounded-xl border border-border/50 p-4 hover:border-border transition-colors cursor-pointer"
              onClick={() => setSelectedTask(task)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                      <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium border', taskTypeColors[task.task_type])}>
                        {taskTypeLabels[task.task_type]}
                      </span>
                      <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium border', taskStatusColors[task.status])}>
                        {taskStatusLabels[task.status]}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground font-mono">{task.id}</p>
                  </div>
                </div>
                <div className="flex items-center gap-4 text-sm text-muted-foreground">
                  <div className="flex items-center gap-1">
                    <Clock className="w-3.5 h-3.5" />
                    <span>{new Date(task.execute_at).toLocaleTimeString()}</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>{task.retry_count}/{task.max_retries}</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Modals */}
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
        />
      )}

      {/* Confirm Modal */}
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