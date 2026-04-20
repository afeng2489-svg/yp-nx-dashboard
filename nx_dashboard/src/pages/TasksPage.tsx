import { useEffect, useState } from 'react';
import { useTasksQuery } from '@/hooks/useReactQuery';
import { useTaskStore, TaskType, TaskStatus, taskTypeLabels, taskStatusLabels, taskStatusColors, taskTypeColors } from '@/stores/taskStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { Clock, Play, Trash2, Plus, X, RefreshCw, CheckCircle, AlertCircle, Timer, List, GitBranch, Shield, Code, Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';
import { API_BASE_URL } from '@/api/constants';

// ===================== 创建任务弹窗 =====================

interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (taskType: TaskType, payload: Record<string, unknown>, delaySeconds?: number, maxRetries?: number) => void;
}

function CreateTaskModal({ isOpen, onClose, onSubmit }: CreateTaskModalProps) {
  const [taskType, setTaskType] = useState<TaskType>('workflow_execution');
  const [delaySeconds, setDelaySeconds] = useState('');
  const [maxRetries, setMaxRetries] = useState('3');

  // workflow_execution fields
  const [selectedWorkflowId, setSelectedWorkflowId] = useState('');
  const [workflowVariables, setWorkflowVariables] = useState('{}');

  // code_review fields
  const [repoUrl, setRepoUrl] = useState('');

  // security_audit fields
  const [auditTarget, setAuditTarget] = useState('');

  // cleanup fields
  const [cleanupType, setCleanupType] = useState('logs');

  const { workflows, fetchWorkflows } = useWorkflowStore();

  useEffect(() => {
    if (isOpen) fetchWorkflows();
  }, [isOpen, fetchWorkflows]);

  if (!isOpen) return null;

  const handleSubmit = async () => {
    let payload: Record<string, unknown>;

    if (taskType === 'workflow_execution') {
      if (!selectedWorkflowId) {
        showError('请选择一个工作流');
        return;
      }
      let vars: Record<string, unknown> = {};
      try {
        vars = JSON.parse(workflowVariables);
      } catch {
        showError('变量格式错误，请输入合法 JSON');
        return;
      }
      // 获取完整工作流 definition 作为 workflow_yaml（JSON 是合法 YAML）
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${selectedWorkflowId}`);
        const wfFull = await res.json();
        payload = {
          workflow_id: selectedWorkflowId,
          workflow_yaml: JSON.stringify(wfFull.definition ?? {}),
          variables: vars,
        };
      } catch {
        showError('获取工作流详情失败，请重试');
        return;
      }
    } else if (taskType === 'code_review') {
      if (!repoUrl.trim()) {
        showError('请输入仓库地址');
        return;
      }
      payload = { repo_url: repoUrl.trim() };
    } else if (taskType === 'security_audit') {
      if (!auditTarget.trim()) {
        showError('请输入审计目标');
        return;
      }
      payload = { target: auditTarget.trim() };
    } else {
      payload = { cleanup_type: cleanupType };
    }

    onSubmit(
      taskType,
      payload,
      delaySeconds ? parseInt(delaySeconds, 10) : undefined,
      maxRetries ? parseInt(maxRetries, 10) : undefined,
    );
    onClose();
  };

  const typeIcons: Record<TaskType, React.ReactNode> = {
    workflow_execution: <GitBranch className="w-4 h-4" />,
    code_review: <Code className="w-4 h-4" />,
    security_audit: <Shield className="w-4 h-4" />,
    cleanup: <Sparkles className="w-4 h-4" />,
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-2xl shadow-2xl border border-border/50 p-6 animate-in fade-in zoom-in duration-200">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <Plus className="w-5 h-5 text-indigo-500" />
            创建后台任务
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-accent rounded-lg transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="space-y-4">
          {/* 任务类型 */}
          <div>
            <label className="block text-sm font-medium mb-2">任务类型</label>
            <div className="grid grid-cols-2 gap-2">
              {(['workflow_execution', 'code_review', 'security_audit', 'cleanup'] as TaskType[]).map((t) => (
                <button
                  key={t}
                  onClick={() => setTaskType(t)}
                  className={cn(
                    'flex items-center gap-2 px-3 py-2.5 rounded-xl border text-sm font-medium transition-all',
                    taskType === t
                      ? 'border-indigo-500/60 bg-indigo-500/10 text-indigo-600'
                      : 'border-border/50 bg-card hover:border-indigo-500/30 hover:bg-indigo-500/5'
                  )}
                >
                  {typeIcons[t]}
                  {taskTypeLabels[t]}
                </button>
              ))}
            </div>
          </div>

          {/* 动态表单 */}
          {taskType === 'workflow_execution' && (
            <>
              <div>
                <label className="block text-sm font-medium mb-2">选择工作流</label>
                <select
                  value={selectedWorkflowId}
                  onChange={(e) => setSelectedWorkflowId(e.target.value)}
                  className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
                >
                  <option value="">-- 请选择 --</option>
                  {workflows.map((wf) => (
                    <option key={wf.id} value={wf.id}>{wf.name}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">输入变量（JSON）</label>
                <textarea
                  value={workflowVariables}
                  onChange={(e) => setWorkflowVariables(e.target.value)}
                  rows={3}
                  className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 font-mono text-sm"
                  placeholder='{"key": "value"}'
                />
              </div>
            </>
          )}

          {taskType === 'code_review' && (
            <div>
              <label className="block text-sm font-medium mb-2">仓库地址</label>
              <input
                type="text"
                value={repoUrl}
                onChange={(e) => setRepoUrl(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
                placeholder="https://github.com/org/repo"
              />
            </div>
          )}

          {taskType === 'security_audit' && (
            <div>
              <label className="block text-sm font-medium mb-2">审计目标（路径或 URL）</label>
              <input
                type="text"
                value={auditTarget}
                onChange={(e) => setAuditTarget(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
                placeholder="/path/to/code 或 https://target.com"
              />
            </div>
          )}

          {taskType === 'cleanup' && (
            <div>
              <label className="block text-sm font-medium mb-2">清理类型</label>
              <select
                value={cleanupType}
                onChange={(e) => setCleanupType(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
              >
                <option value="logs">日志清理</option>
                <option value="temp">临时文件清理</option>
                <option value="cache">缓存清理</option>
              </select>
            </div>
          )}

          {/* 延迟 & 重试 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-2">延迟执行（秒）</label>
              <input
                type="number"
                value={delaySeconds}
                onChange={(e) => setDelaySeconds(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
                placeholder="0（立即执行）"
                min="0"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-2">最大重试次数</label>
              <input
                type="number"
                value={maxRetries}
                onChange={(e) => setMaxRetries(e.target.value)}
                className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
                placeholder="3"
                min="0"
              />
            </div>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 mt-6">
          <button onClick={onClose} className="btn-secondary">取消</button>
          <button onClick={handleSubmit} className="btn-primary">
            <Play className="w-4 h-4" />
            提交任务
          </button>
        </div>
      </div>
    </div>
  );
}

// ===================== 任务详情面板 =====================

interface TaskDetailPanelProps {
  task: ReturnType<typeof useTaskStore.getState>['tasks'][0];
  onClose: () => void;
  onCancel: (id: string) => void;
}

function TaskDetailPanel({ task, onClose, onCancel }: TaskDetailPanelProps) {
  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <List className="w-5 h-5 text-indigo-500" />
            任务详情
          </h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          <div className="flex items-center gap-2 flex-wrap">
            <span className={cn('px-3 py-1 rounded-full text-xs font-medium border', taskTypeColors[task.task_type])}>
              {taskTypeLabels[task.task_type]}
            </span>
            <span className={cn('px-3 py-1 rounded-full text-xs font-medium border', taskStatusColors[task.status])}>
              {taskStatusLabels[task.status]}
            </span>
          </div>

          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">任务 ID</h4>
              <p className="text-sm font-mono bg-accent/50 rounded px-3 py-2 break-all">{task.id}</p>
            </div>

            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">任务参数</h4>
              <pre className="text-sm font-mono bg-accent/50 rounded px-3 py-2 overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(task.payload, null, 2)}
              </pre>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">计划时间</h4>
                <p className="text-sm">{new Date(task.scheduled_at).toLocaleString('zh-CN')}</p>
              </div>
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">执行时间</h4>
                <p className="text-sm">{new Date(task.execute_at).toLocaleString('zh-CN')}</p>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">重试次数</h4>
                <p className="text-sm">{task.retry_count} / {task.max_retries}</p>
              </div>
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">最后更新</h4>
                <p className="text-sm">{new Date(task.updated_at).toLocaleString('zh-CN')}</p>
              </div>
            </div>

            {task.error_message && (
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">错误信息</h4>
                <p className="text-sm text-red-500 bg-red-500/10 rounded px-3 py-2">{task.error_message}</p>
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          {task.status === 'pending' && (
            <button
              onClick={() => onCancel(task.id)}
              className="btn-secondary text-red-500 hover:bg-red-500/10 flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" />
              取消任务
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ===================== 主页面 =====================

export function TasksPage() {
  const { error, createTask, cancelTask } = useTaskStore();
  const [selectedTask, setSelectedTask] = useState<ReturnType<typeof useTaskStore.getState>['tasks'][0] | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const { tasks, stats, loading, refetch } = useTasksQuery();

  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => { refetch(); });
    return () => { unsubscribe(); };
  }, [refetch]);

  const filteredTasks = filter === 'all' ? tasks : tasks.filter((t) => t.status === filter);

  const handleCreateTask = async (taskType: TaskType, payload: Record<string, unknown>, delaySeconds?: number, maxRetries?: number) => {
    await createTask({ task_type: taskType, payload, delay_seconds: delaySeconds, max_retries: maxRetries });
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
      'danger'
    );
  };

  const statusFilterLabels: Record<TaskStatus | 'all', string> = {
    all: '全部',
    pending: '等待中',
    running: '运行中',
    completed: '已完成',
    failed: '已失败',
    cancelled: '已取消',
  };

  return (
    <div className="page-container space-y-6">
      {/* 页头 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              后台任务
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">管理和监控后台定时任务</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => refetch()} className="btn-secondary flex items-center gap-2">
            <RefreshCw className="w-4 h-4" />
            刷新
          </button>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary flex items-center gap-2">
            <Plus className="w-4 h-4" />
            创建任务
          </button>
        </div>
      </div>

      {/* 统计卡片 */}
      {stats && (
        <div className="grid grid-cols-4 gap-4">
          {[
            { label: '等待中', value: stats.pending, icon: Timer, color: 'text-yellow-500', bg: 'bg-yellow-500/10' },
            { label: '运行中', value: stats.running, icon: Play, color: 'text-blue-500', bg: 'bg-blue-500/10' },
            { label: '已完成', value: stats.completed, icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10' },
            { label: '已失败', value: stats.failed, icon: AlertCircle, color: 'text-red-500', bg: 'bg-red-500/10' },
          ].map(({ label, value, icon: Icon, color, bg }) => (
            <div key={label} className="bg-card rounded-xl border border-border/50 p-4">
              <div className="flex items-center gap-3">
                <div className={cn('p-2 rounded-lg', bg)}>
                  <Icon className={cn('w-5 h-5', color)} />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">{label}</p>
                  <p className="text-2xl font-bold">{value}</p>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 状态筛选 */}
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-sm text-muted-foreground">筛选：</span>
        {(['all', 'pending', 'running', 'completed', 'failed', 'cancelled'] as (TaskStatus | 'all')[]).map((status) => (
          <button
            key={status}
            onClick={() => setFilter(status)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-colors',
              filter === status
                ? status === 'all'
                  ? 'bg-indigo-500/10 text-indigo-600 border border-indigo-500/30'
                  : taskStatusColors[status as TaskStatus]
                : 'bg-accent hover:bg-accent/80 border border-transparent'
            )}
          >
            {statusFilterLabels[status]}
          </button>
        ))}
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-4 flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0" />
          <p className="text-sm text-red-500">{error}</p>
        </div>
      )}

      {/* 骨架屏 */}
      {loading && tasks.length === 0 && (
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map(i => <div key={i} className="h-20 bg-muted rounded-xl" />)}
        </div>
      )}

      {/* 任务列表 */}
      {filteredTasks.length === 0 && !loading ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <List className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无任务</h3>
          <p className="text-muted-foreground mb-4 text-sm">
            {filter === 'all' ? '点击「创建任务」提交一个后台任务' : `当前没有「${statusFilterLabels[filter]}」状态的任务`}
          </p>
          {filter === 'all' && (
            <button onClick={() => setShowCreateModal(true)} className="btn-primary flex items-center gap-2 mx-auto">
              <Plus className="w-4 h-4" />
              创建任务
            </button>
          )}
        </div>
      ) : (
        <div className="space-y-3">
          {filteredTasks.map((task) => (
            <div
              key={task.id}
              onClick={() => setSelectedTask(task)}
              className="bg-card rounded-xl border border-border/50 p-4 hover:border-indigo-500/30 hover:shadow-sm transition-all cursor-pointer group"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4 min-w-0">
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center gap-2">
                      <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium border', taskTypeColors[task.task_type])}>
                        {taskTypeLabels[task.task_type]}
                      </span>
                      <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium border', taskStatusColors[task.status])}>
                        {taskStatusLabels[task.status]}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground font-mono truncate max-w-xs">{task.id}</p>
                  </div>
                </div>
                <div className="flex items-center gap-5 text-sm text-muted-foreground shrink-0">
                  <div className="flex items-center gap-1.5">
                    <Clock className="w-3.5 h-3.5" />
                    <span>{new Date(task.execute_at).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>{task.retry_count}/{task.max_retries} 次重试</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 弹窗 */}
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

      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={() => { confirmState.onConfirm(); hideConfirm(); }}
        onCancel={hideConfirm}
        variant={confirmState.variant || 'danger'}
      />
    </div>
  );
}
