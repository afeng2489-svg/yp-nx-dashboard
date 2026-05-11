import { RotateCcw } from 'lucide-react';
import { cn } from '@/lib/utils';

export function RollbackActions({
  rollbackAction,
  rollingBack,
  onSelectAction,
  onRollback,
}: {
  rollbackAction: 'revert' | 'keep' | 'branch';
  rollingBack: boolean;
  onSelectAction: (action: 'revert' | 'keep' | 'branch') => void;
  onRollback: () => void;
}) {
  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-destructive">执行失败 — 回滚选项</h3>
      <div className="flex gap-2">
        {(['revert', 'keep', 'branch'] as const).map((action) => (
          <button
            key={action}
            onClick={() => onSelectAction(action)}
            className={cn(
              'px-3 py-2 text-sm rounded-lg border transition-colors',
              rollbackAction === action
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-border hover:bg-accent',
            )}
          >
            {action === 'revert' && '回滚到执行前'}
            {action === 'keep' && '保留当前分支'}
            {action === 'branch' && '创建 fix 分支'}
          </button>
        ))}
      </div>
      <button
        onClick={onRollback}
        disabled={rollingBack}
        className="flex items-center gap-2 px-4 py-2 text-sm rounded-lg bg-destructive text-white hover:bg-destructive/90 disabled:opacity-50"
      >
        <RotateCcw className="w-4 h-4" />
        {rollingBack ? '回滚中...' : '执行回滚'}
      </button>
    </div>
  );
}
