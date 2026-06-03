import { useState } from 'react';
import { Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { IssuePriority, issuePriorityLabels, issuePriorityColors } from '@/stores/issueStore';
import { showError } from '@/lib/toast';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import { FormField, FormSection, formControlClass, formTextareaClass } from '@/components/ui/formStyles';

interface CreateIssueModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (
    title: string,
    description: string,
    priority: IssuePriority,
    perspectives: string[],
  ) => void;
}

const PERSPECTIVE_OPTIONS = ['bug', 'security', 'performance', 'maintainability'];

export function CreateIssueModal({ isOpen, onClose, onSubmit }: CreateIssueModalProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<IssuePriority>('medium');
  const [perspectives, setPerspectives] = useState<string[]>([]);

  const togglePerspective = (p: string) => {
    setPerspectives((prev) => (prev.includes(p) ? prev.filter((x) => x !== p) : [...prev, p]));
  };

  const handleSubmit = () => {
    if (!title.trim()) {
      showError('请输入标题');
      return;
    }
    if (!description.trim()) {
      showError('请输入描述');
      return;
    }
    onSubmit(title.trim(), description.trim(), priority, perspectives);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <LaunchModalShell
      onClose={onClose}
      title="新建 Issue"
      subtitle="记录问题并选择审查视角"
      size="lg"
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={handleSubmit}
          submitLabel="创建"
          submitIcon={<Plus className="h-4 w-4" />}
        />
      }
    >
      <div className="space-y-8">
        <FormSection title="问题描述">
          <FormField label="标题" required>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className={formControlClass}
              placeholder="简短描述问题"
            />
          </FormField>
          <FormField label="详细描述" required>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={formTextareaClass}
              placeholder="影响范围、重现步骤等"
              rows={4}
            />
          </FormField>
        </FormSection>

        <FormSection title="分类">
          <FormField label="优先级">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(['critical', 'high', 'medium', 'low'] as IssuePriority[]).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => setPriority(p)}
                  className={cn(
                    'h-10 rounded-lg border text-xs font-medium transition-colors sm:text-sm',
                    priority === p
                      ? issuePriorityColors[p]
                      : 'border-border bg-background text-muted-foreground hover:bg-muted/50',
                  )}
                >
                  {issuePriorityLabels[p]}
                </button>
              ))}
            </div>
          </FormField>
          <FormField label="视角标签">
            <div className="flex flex-wrap gap-2">
              {PERSPECTIVE_OPTIONS.map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => togglePerspective(p)}
                  className={cn(
                    'rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
                    perspectives.includes(p)
                      ? 'border-primary bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground hover:bg-muted/50',
                  )}
                >
                  {p}
                </button>
              ))}
            </div>
          </FormField>
        </FormSection>
      </div>
    </LaunchModalShell>
  );
}
