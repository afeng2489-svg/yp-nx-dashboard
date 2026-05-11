import { useState } from 'react';
import { Plus, X, Play, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { TaskPriority, ExecutionMode, taskPriorityLabels } from '@/stores/taskStore';
import { showError } from '@/lib/toast';

interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (
    name: string,
    description: string,
    priority: TaskPriority,
    executionMode: ExecutionMode,
  ) => Promise<void> | void;
}

export function CreateTaskModal({ isOpen, onClose, onSubmit }: CreateTaskModalProps) {
  const [taskName, setTaskName] = useState('');
  const [taskDescription, setTaskDescription] = useState('');
  const [priority, setPriority] = useState<TaskPriority>('normal');
  const [executionMode, setExecutionMode] = useState<ExecutionMode>('auto_plan');
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = async () => {
    if (!taskName.trim()) {
      showError('请输入任务名称');
      return;
    }
    setIsSubmitting(true);
    try {
      await onSubmit(taskName.trim(), taskDescription.trim(), priority, executionMode);
      onClose();
    } catch (e) {
      showError(`提交失败: ${(e as Error).message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg max-h-[90vh] overflow-y-auto bg-card rounded-2xl shadow-2xl border border-border/50 p-6 animate-in fade-in zoom-in duration-200">
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
          <div>
            <label className="block text-sm font-medium mb-2">任务名称</label>
            <input
              type="text"
              value={taskName}
              onChange={(e) => setTaskName(e.target.value)}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
              placeholder="如：修复登录 Bug"
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">任务描述</label>
            <textarea
              value={taskDescription}
              onChange={(e) => setTaskDescription(e.target.value)}
              rows={3}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm"
              placeholder="详细描述任务需求..."
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">优先级</label>
            <div className="grid grid-cols-4 gap-2">
              {(['low', 'normal', 'high', 'critical'] as TaskPriority[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setPriority(p)}
                  className={cn(
                    'px-3 py-2 rounded-xl border text-sm font-medium transition-all',
                    priority === p
                      ? 'border-indigo-500/60 bg-indigo-500/10 text-indigo-600'
                      : 'border-border/50 bg-card hover:border-indigo-500/30',
                  )}
                >
                  {taskPriorityLabels[p]}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">执行模式</label>
            <div className="grid grid-cols-3 gap-2">
              {[
                {
                  value: 'auto_plan' as ExecutionMode,
                  label: 'AI 自动规划',
                  desc: 'Planner 自动生成执行计划',
                },
                {
                  value: 'workflow' as ExecutionMode,
                  label: '预定义工作流',
                  desc: '选择已有工作流执行',
                },
                {
                  value: 'manual' as ExecutionMode,
                  label: '手动模式',
                  desc: '仅入队，手动指定 stages',
                },
              ].map((mode) => (
                <button
                  key={mode.value}
                  onClick={() => setExecutionMode(mode.value)}
                  className={cn(
                    'px-3 py-2 rounded-xl border text-sm font-medium transition-all text-left',
                    executionMode === mode.value
                      ? 'border-indigo-500/60 bg-indigo-500/10 text-indigo-600'
                      : 'border-border/50 bg-card hover:border-indigo-500/30',
                  )}
                >
                  <div>{mode.label}</div>
                  <div className="text-xs opacity-60 mt-0.5">{mode.desc}</div>
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 mt-6">
          <button onClick={onClose} disabled={isSubmitting} className="btn-secondary">
            取消
          </button>
          <button onClick={handleSubmit} disabled={isSubmitting} className="btn-primary">
            {isSubmitting ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Play className="w-4 h-4" />
            )}
            {isSubmitting ? '提交中...' : '提交任务'}
          </button>
        </div>
      </div>
    </div>
  );
}
