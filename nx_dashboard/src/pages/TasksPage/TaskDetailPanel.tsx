import { X, List, CheckCircle, AlertCircle, RefreshCw, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  useTaskStore,
  TaskStatus,
  taskPriorityLabels,
  taskStatusLabels,
  taskStatusColors,
  taskPriorityColors,
} from '@/stores/taskStore';

interface TaskDetailPanelProps {
  task: ReturnType<typeof useTaskStore.getState>['tasks'][0];
  onClose: () => void;
  onCancel: (id: string) => void;
  onUpdateStatus: (id: string, status: TaskStatus) => void;
}

export function TaskDetailPanel({ task, onClose, onCancel, onUpdateStatus }: TaskDetailPanelProps) {
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
            <span
              className={cn(
                'px-3 py-1 rounded-full text-xs font-medium border',
                taskPriorityColors[task.priority],
              )}
            >
              {taskPriorityLabels[task.priority]}
            </span>
            <span
              className={cn(
                'px-3 py-1 rounded-full text-xs font-medium border',
                taskStatusColors[task.status],
              )}
            >
              {taskStatusLabels[task.status]}
            </span>
          </div>

          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">任务名称</h4>
              <p className="text-sm">{task.name}</p>
            </div>
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-1">任务 ID</h4>
              <p className="text-sm font-mono bg-accent/50 rounded px-3 py-2 break-all">
                {task.id}
              </p>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">创建时间</h4>
                <p className="text-sm">{new Date(task.created_at).toLocaleString('zh-CN')}</p>
              </div>
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">开始时间</h4>
                <p className="text-sm">
                  {task.started_at ? new Date(task.started_at).toLocaleString('zh-CN') : '—'}
                </p>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">完成时间</h4>
                <p className="text-sm">
                  {task.finished_at ? new Date(task.finished_at).toLocaleString('zh-CN') : '—'}
                </p>
              </div>
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">重试次数</h4>
                <p className="text-sm">{task.retry_count}</p>
              </div>
            </div>
            {task.error && (
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">错误信息</h4>
                <p className="text-sm text-red-500 bg-red-500/10 rounded px-3 py-2">{task.error}</p>
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center justify-between px-6 py-4 border-t border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <div className="flex items-center gap-2">
            {task.status === 'running' && (
              <>
                <button
                  onClick={() => onUpdateStatus(task.id, 'completed')}
                  className="btn-secondary text-green-600 hover:bg-green-500/10 flex items-center gap-2 text-sm"
                >
                  <CheckCircle className="w-4 h-4" />
                  标记完成
                </button>
                <button
                  onClick={() => onUpdateStatus(task.id, 'failed')}
                  className="btn-secondary text-orange-500 hover:bg-orange-500/10 flex items-center gap-2 text-sm"
                >
                  <AlertCircle className="w-4 h-4" />
                  标记失败
                </button>
              </>
            )}
            {(task.status === 'failed' || task.status === 'cancelled') && (
              <button
                onClick={() => onUpdateStatus(task.id, 'queued')}
                className="btn-secondary text-blue-500 hover:bg-blue-500/10 flex items-center gap-2 text-sm"
              >
                <RefreshCw className="w-4 h-4" />
                重新排队
              </button>
            )}
          </div>
          {(task.status === 'queued' || task.status === 'delayed') && (
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
