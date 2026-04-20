import { X, Pause, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useExecutionStore } from '@/stores/executionStore';

function PauseCardContent() {
  const { pendingPause, resumeExecution, dismissPause } = useExecutionStore();
  if (!pendingPause) return null;

  return (
    <div className="w-80 rounded-2xl border border-amber-500/30 bg-card shadow-2xl shadow-amber-500/10 overflow-hidden">
      <div className="h-1 bg-gradient-to-r from-amber-400 to-orange-400" />

      <div className="p-4">
        <div className="flex items-start gap-3 mb-3">
          <div className="w-8 h-8 rounded-xl bg-amber-500/15 flex items-center justify-center flex-shrink-0">
            <Pause className="w-4 h-4 text-amber-500" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <p className="text-sm font-semibold text-amber-600 dark:text-amber-400">
                工作流等待输入
              </p>
              <button
                onClick={dismissPause}
                className="p-0.5 rounded-md hover:bg-accent text-muted-foreground transition-colors flex-shrink-0"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5 truncate">
              {pendingPause.stage_name}
            </p>
          </div>
        </div>

        <p className="text-sm text-foreground mb-3 leading-relaxed">
          {pendingPause.question}
        </p>

        <div className="space-y-1.5">
          {pendingPause.options.map((opt) => (
            <button
              key={opt.value}
              onClick={() => resumeExecution(pendingPause.execution_id, opt.value)}
              className={cn(
                'w-full flex items-center justify-between gap-2 px-3 py-2 rounded-xl text-sm',
                'border border-border bg-accent/50 hover:border-amber-500/40 hover:bg-amber-500/5',
                'transition-all text-left group'
              )}
            >
              <span className="font-medium">{opt.label}</span>
              <ChevronRight className="w-3.5 h-3.5 text-muted-foreground group-hover:text-amber-500 transition-colors flex-shrink-0" />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

/** 用于 GlobalOpsOverlay 堆叠容器内（无 fixed） */
export function WorkflowPauseCardInline() {
  return <PauseCardContent />;
}

/** 独立使用时带 fixed 定位 */
export function WorkflowPauseCard() {
  const pendingPause = useExecutionStore((s) => s.pendingPause);
  if (!pendingPause) return null;
  return (
    <div className="fixed bottom-4 right-4 z-50">
      <PauseCardContent />
    </div>
  );
}
