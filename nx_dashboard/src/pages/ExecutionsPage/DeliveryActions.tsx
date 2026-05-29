import { useState } from 'react';
import { GitMerge, Trash2, Loader2, Copy } from 'lucide-react';
import { cn } from '@/lib/utils';

type Decision = 'merge' | 'revert';

export function DeliveryActions({
  busy,
  onDecide,
  onCopyPr,
  copied,
}: {
  busy: boolean;
  onDecide: (action: Decision) => void;
  onCopyPr: () => void;
  copied: boolean;
}) {
  const [pending, setPending] = useState<Decision | null>(null);

  const decide = (action: Decision) => {
    setPending(action);
    onDecide(action);
  };

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-foreground">交付审批 — 处理本次改动</h3>
      <div className="flex flex-wrap gap-2">
        <button
          onClick={() => decide('merge')}
          disabled={busy}
          className={cn(
            'flex items-center gap-2 px-4 py-2 text-sm rounded-lg text-white transition-colors disabled:opacity-50',
            'bg-emerald-600 hover:bg-emerald-600/90',
          )}
        >
          {busy && pending === 'merge' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <GitMerge className="w-4 h-4" />
          )}
          采纳并合并
        </button>
        <button
          onClick={() => decide('revert')}
          disabled={busy}
          className={cn(
            'flex items-center gap-2 px-4 py-2 text-sm rounded-lg border transition-colors disabled:opacity-50',
            'border-destructive/50 text-destructive hover:bg-destructive/10',
          )}
        >
          {busy && pending === 'revert' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Trash2 className="w-4 h-4" />
          )}
          丢弃改动
        </button>
        <button
          onClick={onCopyPr}
          disabled={busy}
          className="flex items-center gap-2 px-4 py-2 text-sm rounded-lg border border-border hover:bg-accent transition-colors disabled:opacity-50"
        >
          <Copy className="w-4 h-4" />
          {copied ? '已复制!' : '复制 PR 描述'}
        </button>
      </div>
      <p className="text-xs text-muted-foreground">
        采纳：将执行分支合并回原分支并删除执行分支。丢弃：切回原分支并删除执行分支，回到执行前状态。
      </p>
    </div>
  );
}
