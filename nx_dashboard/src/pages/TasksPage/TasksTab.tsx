import {
  TaskStatus,
  TaskPriority,
  taskStatusLabels,
  taskStatusColors,
  taskPriorityLabels,
  taskPriorityColors,
} from '@/stores/taskStore';
import { Clock, Play, Plus, RefreshCw, CheckCircle, AlertCircle, Timer, List } from 'lucide-react';
import { cn } from '@/lib/utils';

interface TaskStats {
  queued: number;
  running: number;
  completed: number;
  failed: number;
}

interface Task {
  id: string;
  name: string;
  status: TaskStatus;
  priority: TaskPriority;
  created_at: string;
  retry_count: number;
}

interface TasksTabProps {
  tasks: Task[];
  stats: TaskStats | null;
  loading: boolean;
  error: string | null;
  filter: TaskStatus | 'all';
  onFilterChange: (filter: TaskStatus | 'all') => void;
  onTaskClick: (task: Task) => void;
  onCreateClick: () => void;
}

const STATUS_FILTER_OPTIONS: Array<TaskStatus | 'all'> = [
  'all',
  'queued',
  'running',
  'completed',
  'failed',
  'cancelled',
];

const STATUS_FILTER_LABELS: Record<TaskStatus | 'all', string> = {
  all: '全部',
  queued: '等待中',
  running: '运行中',
  completed: '已完成',
  failed: '已失败',
  cancelled: '已取消',
  delayed: '延迟中',
  timed_out: '已超时',
};

const STAT_CARDS = [
  {
    label: '等待中',
    key: 'queued' as const,
    icon: Timer,
    color: 'text-yellow-500',
    bg: 'bg-yellow-500/10',
  },
  {
    label: '运行中',
    key: 'running' as const,
    icon: Play,
    color: 'text-blue-500',
    bg: 'bg-blue-500/10',
  },
  {
    label: '已完成',
    key: 'completed' as const,
    icon: CheckCircle,
    color: 'text-green-500',
    bg: 'bg-green-500/10',
  },
  {
    label: '已失败',
    key: 'failed' as const,
    icon: AlertCircle,
    color: 'text-red-500',
    bg: 'bg-red-500/10',
  },
];

export function TasksTab({
  tasks,
  stats,
  loading,
  error,
  filter,
  onFilterChange,
  onTaskClick,
  onCreateClick,
}: TasksTabProps) {
  const filteredTasks = filter === 'all' ? tasks : tasks.filter((t) => t.status === filter);

  return (
    <>
      {stats && (
        <div className="grid grid-cols-4 gap-4">
          {STAT_CARDS.map(({ label, key, icon: Icon, color, bg }) => (
            <div key={label} className="bg-card rounded-xl border border-border/50 p-4">
              <div className="flex items-center gap-3">
                <div className={cn('p-2 rounded-lg', bg)}>
                  <Icon className={cn('w-5 h-5', color)} />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">{label}</p>
                  <p className="text-2xl font-bold">{stats[key]}</p>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-sm text-muted-foreground">筛选：</span>
        {STATUS_FILTER_OPTIONS.map((status) => (
          <button
            key={status}
            onClick={() => onFilterChange(status)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-colors',
              filter === status
                ? status === 'all'
                  ? 'bg-indigo-500/10 text-indigo-600 border border-indigo-500/30'
                  : taskStatusColors[status as TaskStatus]
                : 'bg-accent hover:bg-accent/80 border border-transparent',
            )}
          >
            {STATUS_FILTER_LABELS[status]}
          </button>
        ))}
      </div>

      {error && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-4 flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0" />
          <p className="text-sm text-red-500">{error}</p>
        </div>
      )}

      {loading && tasks.length === 0 && (
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-20 bg-muted rounded-xl" />
          ))}
        </div>
      )}

      {filteredTasks.length === 0 && !loading ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <List className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无任务</h3>
          <p className="text-muted-foreground mb-4 text-sm">
            {filter === 'all'
              ? '点击「创建任务」提交一个后台任务'
              : `当前没有「${STATUS_FILTER_LABELS[filter]}」状态的任务`}
          </p>
          {filter === 'all' && (
            <button onClick={onCreateClick} className="btn-primary flex items-center gap-2 mx-auto">
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
              onClick={() => onTaskClick(task)}
              className="bg-card rounded-xl border border-border/50 p-4 hover:border-indigo-500/30 hover:shadow-sm transition-all cursor-pointer group"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4 min-w-0">
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          'px-2 py-0.5 rounded-full text-xs font-medium border',
                          taskPriorityColors[task.priority],
                        )}
                      >
                        {taskPriorityLabels[task.priority]}
                      </span>
                      <span
                        className={cn(
                          'px-2 py-0.5 rounded-full text-xs font-medium border',
                          taskStatusColors[task.status],
                        )}
                      >
                        {taskStatusLabels[task.status]}
                      </span>
                    </div>
                    <p className="text-sm font-medium truncate">{task.name}</p>
                    <p className="text-xs text-muted-foreground font-mono truncate max-w-xs">
                      {task.id}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-5 text-sm text-muted-foreground shrink-0">
                  <div className="flex items-center gap-1.5">
                    <Clock className="w-3.5 h-3.5" />
                    <span>
                      {new Date(task.created_at).toLocaleString('zh-CN', {
                        month: 'numeric',
                        day: 'numeric',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>{task.retry_count} 次重试</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
