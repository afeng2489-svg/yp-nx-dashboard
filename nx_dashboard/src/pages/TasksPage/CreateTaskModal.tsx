import { useState } from 'react';
import { Play, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { TaskPriority, ExecutionMode, taskPriorityLabels } from '@/stores/taskStore';
import { showError } from '@/lib/toast';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import { FormField, FormSection, formControlClass, formTextareaClass } from '@/components/ui/formStyles';

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

  if (!isOpen) return null;

  return (
    <LaunchModalShell
      onClose={onClose}
      title="创建后台任务"
      subtitle="任务将加入队列，由 Agent 按所选模式执行"
      size="lg"
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={handleSubmit}
          submitLabel="提交任务"
          submitting={isSubmitting}
          submitIcon={!isSubmitting ? <Play className="h-4 w-4" /> : undefined}
        />
      }
    >
      <div className="space-y-8">
        <FormSection title="任务信息">
          <FormField label="任务名称" required>
            <input
              type="text"
              value={taskName}
              onChange={(e) => setTaskName(e.target.value)}
              className={formControlClass}
              placeholder="如：修复登录 Bug"
            />
          </FormField>
          <FormField label="任务描述">
            <textarea
              value={taskDescription}
              onChange={(e) => setTaskDescription(e.target.value)}
              className={formTextareaClass}
              placeholder="详细描述任务需求…"
              rows={3}
            />
          </FormField>
        </FormSection>

        <FormSection title="执行配置">
          <FormField label="优先级">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(['low', 'normal', 'high', 'critical'] as TaskPriority[]).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => setPriority(p)}
                  className={cn(
                    'h-10 rounded-lg border text-sm font-medium transition-colors',
                    priority === p
                      ? 'border-primary bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground hover:bg-muted/50',
                  )}
                >
                  {taskPriorityLabels[p]}
                </button>
              ))}
            </div>
          </FormField>
          <FormField label="执行模式">
            <div className="grid gap-2 sm:grid-cols-3">
              {[
                { value: 'auto_plan' as ExecutionMode, label: 'AI 自动规划', desc: '自动生成计划' },
                { value: 'workflow' as ExecutionMode, label: '预定义工作流', desc: '选择已有流程' },
                { value: 'manual' as ExecutionMode, label: '手动模式', desc: '手动指定阶段' },
              ].map((mode) => (
                <button
                  key={mode.value}
                  type="button"
                  onClick={() => setExecutionMode(mode.value)}
                  className={cn(
                    'rounded-lg border px-3 py-2.5 text-left text-sm transition-colors',
                    executionMode === mode.value
                      ? 'border-primary bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground hover:bg-muted/50',
                  )}
                >
                  <div className="font-medium">{mode.label}</div>
                  <div className="mt-0.5 text-xs opacity-70">{mode.desc}</div>
                </button>
              ))}
            </div>
          </FormField>
        </FormSection>
      </div>
    </LaunchModalShell>
  );
}
