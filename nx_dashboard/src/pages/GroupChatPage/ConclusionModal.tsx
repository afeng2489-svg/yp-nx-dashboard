export interface ConclusionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConclude: (force: boolean) => void;
}

export function ConclusionModal({ isOpen, onClose, onConclude }: ConclusionModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
        <h2 className="text-xl font-bold mb-4">结束讨论</h2>
        <p className="text-sm text-muted-foreground mb-4">
          确定要结束当前讨论吗？系统将基于讨论内容生成最终结论。
        </p>
        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="btn btn-outline">
            取消
          </button>
          <button onClick={() => onConclude(false)} className="btn btn-primary">
            正常结束
          </button>
          <button onClick={() => onConclude(true)} className="btn btn-destructive">
            强制结束
          </button>
        </div>
      </div>
    </div>
  );
}
