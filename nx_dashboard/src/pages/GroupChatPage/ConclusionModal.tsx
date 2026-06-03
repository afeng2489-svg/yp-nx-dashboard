import { Flag } from 'lucide-react';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';

export interface ConclusionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConclude: (force: boolean) => void;
}

export function ConclusionModal({ isOpen, onClose, onConclude }: ConclusionModalProps) {
  if (!isOpen) return null;

  return (
    <LaunchModalShell
      onClose={onClose}
      title="结束讨论"
      subtitle="正常结束会等待当前回合完成；强制结束将立即终止"
      icon={<Flag />}
      accent="amber"
      size="md"
      footer={
        <div className="flex flex-col gap-2.5 sm:flex-row sm:justify-end">
          <button type="button" onClick={onClose} className="btn-ghost h-11 px-5 text-sm">
            取消
          </button>
          <button
            type="button"
            onClick={() => onConclude(false)}
            className="btn-primary h-11 px-5 text-sm"
          >
            正常结束
          </button>
          <button
            type="button"
            onClick={() => onConclude(true)}
            className="btn-secondary h-11 border border-destructive/30 px-5 text-sm text-destructive hover:text-destructive"
          >
            强制结束
          </button>
        </div>
      }
    >
      <p className="text-sm leading-relaxed text-muted-foreground">
        结束后系统将基于讨论记录生成结论摘要。若讨论仍在进行中，建议使用「正常结束」。
      </p>
    </LaunchModalShell>
  );
}
