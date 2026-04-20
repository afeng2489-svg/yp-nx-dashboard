import { useState } from 'react';
import { PauseCircle, ChevronRight, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { WorkflowPauseState, WorkflowPauseOption } from '@/stores/executionStore';

interface WorkflowPauseModalProps {
  pause: WorkflowPauseState;
  onResume: (value: string) => boolean;
  onDismiss: () => void;
}

export function WorkflowPauseModal({ pause, onResume, onDismiss }: WorkflowPauseModalProps) {
  const [selected, setSelected] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);

  const handleConfirm = () => {
    if (!selected) return;
    setConfirming(true);
    const ok = onResume(selected);
    if (!ok) setConfirming(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-2xl w-full max-w-lg border border-border/50 overflow-hidden animate-scale-in">
        {/* 头部 */}
        <div className="px-6 pt-6 pb-4">
          <div className="flex items-center gap-3 mb-1">
            <div className="p-2.5 rounded-xl bg-gradient-to-br from-amber-400 to-orange-500 shadow-lg shadow-amber-500/30">
              <PauseCircle className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">工作流等待输入</h2>
              <p className="text-xs text-muted-foreground font-mono">阶段：{pause.stage_name}</p>
            </div>
          </div>
        </div>

        {/* 问题 */}
        <div className="px-6 pb-4">
          <p className="text-base font-medium text-foreground leading-relaxed">
            {pause.question}
          </p>
        </div>

        {/* 选项列表 */}
        <div className="px-6 pb-4 space-y-2.5">
          {pause.options.map((option: WorkflowPauseOption) => (
            <OptionButton
              key={option.value}
              option={option}
              isSelected={selected === option.value}
              onSelect={() => setSelected(option.value)}
            />
          ))}
        </div>

        {/* 操作按钮 */}
        <div className="px-6 py-4 border-t border-border/50 flex items-center gap-3 bg-muted/30">
          <button
            onClick={onDismiss}
            className="px-4 py-2 text-sm rounded-xl text-muted-foreground hover:text-foreground hover:bg-accent transition-all"
          >
            稍后处理
          </button>
          <button
            onClick={handleConfirm}
            disabled={!selected || confirming}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium rounded-xl transition-all',
              selected && !confirming
                ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-lg shadow-amber-500/25 hover:shadow-amber-500/40'
                : 'bg-muted text-muted-foreground cursor-not-allowed'
            )}
          >
            {confirming ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>继续执行中...</span>
              </>
            ) : (
              <>
                <span>确认并继续</span>
                <ChevronRight className="w-4 h-4" />
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

function OptionButton({
  option,
  isSelected,
  onSelect,
}: {
  option: WorkflowPauseOption;
  isSelected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className={cn(
        'w-full flex items-center gap-3 px-4 py-3.5 rounded-xl border text-left transition-all duration-150',
        isSelected
          ? 'border-amber-500/60 bg-gradient-to-r from-amber-500/10 to-orange-500/10 shadow-sm'
          : 'border-border/50 bg-card hover:border-amber-500/30 hover:bg-amber-500/5'
      )}
    >
      {/* 选择指示器 */}
      <div className={cn(
        'w-5 h-5 rounded-full border-2 flex-shrink-0 flex items-center justify-center transition-all',
        isSelected
          ? 'border-amber-500 bg-amber-500'
          : 'border-muted-foreground/40'
      )}>
        {isSelected && (
          <div className="w-2 h-2 rounded-full bg-white" />
        )}
      </div>

      {/* 选项文字 */}
      <span className={cn(
        'text-sm font-medium leading-snug',
        isSelected ? 'text-amber-700 dark:text-amber-300' : 'text-foreground'
      )}>
        {option.label}
      </span>
    </button>
  );
}
