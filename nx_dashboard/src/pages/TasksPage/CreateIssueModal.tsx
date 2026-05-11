import { useState } from 'react';
import { Bug, X, Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { IssuePriority, issuePriorityLabels, issuePriorityColors } from '@/stores/issueStore';
import { showError } from '@/lib/toast';

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

  if (!isOpen) return null;

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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg max-h-[90vh] overflow-y-auto bg-card rounded-2xl shadow-2xl border border-border/50 p-6 animate-in fade-in zoom-in duration-200">
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <Bug className="w-5 h-5 text-red-500" />
            新建 Issue
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-accent rounded-lg transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">标题</label>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-red-500/50 text-sm"
              placeholder="简短描述问题"
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">描述</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={4}
              className="w-full px-4 py-2 rounded-lg border border-border bg-background focus:outline-none focus:ring-2 focus:ring-red-500/50 text-sm resize-none"
              placeholder="详细描述问题、影响范围和重现步骤"
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">优先级</label>
            <div className="grid grid-cols-4 gap-2">
              {(['critical', 'high', 'medium', 'low'] as IssuePriority[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setPriority(p)}
                  className={cn(
                    'px-3 py-1.5 rounded-lg border text-xs font-medium transition-all',
                    priority === p
                      ? issuePriorityColors[p]
                      : 'border-border/50 bg-card hover:bg-accent',
                  )}
                >
                  {issuePriorityLabels[p]}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">视角标签</label>
            <div className="flex flex-wrap gap-2">
              {PERSPECTIVE_OPTIONS.map((p) => (
                <button
                  key={p}
                  onClick={() => togglePerspective(p)}
                  className={cn(
                    'px-3 py-1 rounded-full border text-xs font-medium transition-all',
                    perspectives.includes(p)
                      ? 'bg-indigo-500/10 text-indigo-600 border-indigo-500/30'
                      : 'border-border/50 bg-card hover:bg-accent',
                  )}
                >
                  {p}
                </button>
              ))}
            </div>
          </div>
        </div>
        <div className="flex items-center justify-end gap-3 mt-6">
          <button onClick={onClose} className="btn-secondary">
            取消
          </button>
          <button onClick={handleSubmit} className="btn-primary">
            <Plus className="w-4 h-4" />
            创建
          </button>
        </div>
      </div>
    </div>
  );
}
