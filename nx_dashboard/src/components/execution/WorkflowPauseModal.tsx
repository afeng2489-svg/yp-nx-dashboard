import { PauseCircle, ChevronRight, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { WorkflowPauseState, WorkflowPauseOption } from '@/stores/executionStore';

interface WorkflowPauseModalProps {
  pause: WorkflowPauseState;
  onResume: (value: string) => void;
  onDismiss: () => void;
}

export function WorkflowPauseModal({ pause, onResume, onDismiss }: WorkflowPauseModalProps) {
  return (
    <div className="fixed bottom-20 right-4 z-40 w-80 bg-card rounded-2xl shadow-2xl border border-amber-500/40 overflow-hidden animate-scale-in">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-gradient-to-r from-amber-500/15 to-orange-500/10 border-b border-amber-500/20">
        <div className="flex items-center gap-2">
          <PauseCircle className="w-4 h-4 text-amber-500 animate-pulse flex-shrink-0" />
          <div>
            <p className="text-sm font-semibold leading-none">工作流等待输入</p>
            <p className="text-xs text-muted-foreground mt-0.5 font-mono">{pause.stage_name}</p>
          </div>
        </div>
        <button
          onClick={onDismiss}
          className="p-1 rounded-lg hover:bg-amber-500/20 text-muted-foreground hover:text-foreground transition-colors"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Question */}
      <div className="px-4 pt-3 pb-2">
        <p className="text-sm text-foreground leading-relaxed">{pause.question}</p>
      </div>

      {/* Options — click immediately triggers */}
      <div className="px-4 pb-4 space-y-2">
        {pause.options.map((option: WorkflowPauseOption) => (
          <button
            key={option.value}
            onClick={() => onResume(option.value)}
            className={cn(
              'w-full flex items-center justify-between px-3 py-2.5 rounded-xl border text-left text-sm',
              'border-border/50 bg-card hover:border-amber-500/40 hover:bg-amber-500/5',
              'transition-all duration-150 group',
            )}
          >
            <span className="font-medium group-hover:text-amber-700 dark:group-hover:text-amber-300 transition-colors">
              {option.label}
            </span>
            <ChevronRight className="w-3.5 h-3.5 text-muted-foreground group-hover:text-amber-500 flex-shrink-0 transition-colors" />
          </button>
        ))}
      </div>
    </div>
  );
}
