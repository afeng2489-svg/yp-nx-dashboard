import { X, Users, Loader2, CheckCircle, AlertCircle, Clock } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { cn } from '@/lib/utils';
import { useTeamStore } from '@/stores/teamStore';

function TaskCardContent() {
  const { activeTeamTask, clearActiveTeamTask, stopExecution } = useTeamStore();
  const outputRef = useRef<HTMLDivElement>(null);
  const [elapsed, setElapsed] = useState(0);

  const { teamName, task, status, result, error, partialOutput } = activeTeamTask ?? {};
  const isDone = status === 'done';
  const isError = status === 'error';
  const isRunning = status === 'running';

  // Distinguish server heartbeat placeholder from real CLI output
  const isThinkingPlaceholder =
    isRunning && typeof partialOutput === 'string' && partialOutput.startsWith('思考中...');
  const hasRealOutput = isRunning && !!partialOutput && !isThinkingPlaceholder;
  // Extract elapsed secs from "思考中... (Xs)" placeholder
  const serverElapsed = isThinkingPlaceholder
    ? parseInt(partialOutput!.match(/\((\d+)s\)/)?.[1] ?? '0', 10)
    : null;
  // Show a warning when waiting long with no real output
  const showSlowWarning = isRunning && elapsed > 60 && !hasRealOutput;

  // Local elapsed timer
  useEffect(() => {
    if (!isRunning) {
      setElapsed(0);
      return;
    }
    setElapsed(0);
    const timer = setInterval(() => setElapsed((s) => s + 1), 1000);
    return () => clearInterval(timer);
  }, [isRunning]);

  // Auto-scroll output
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [partialOutput]);

  if (!activeTeamTask) return null;

  return (
    <div
      className={cn(
        'w-80 rounded-2xl border shadow-2xl overflow-hidden bg-card',
        isDone && 'border-emerald-500/30 shadow-emerald-500/10',
        isError && 'border-red-500/30 shadow-red-500/10',
        isRunning && 'border-indigo-500/30 shadow-indigo-500/10',
      )}
    >
      <div
        className={cn(
          'h-1 bg-gradient-to-r',
          isDone && 'from-emerald-400 to-teal-400',
          isError && 'from-red-400 to-rose-400',
          isRunning && 'from-indigo-400 to-purple-400',
        )}
      />

      <div className="p-4">
        <div className="flex items-start gap-3 mb-3">
          <div
            className={cn(
              'w-8 h-8 rounded-xl flex items-center justify-center flex-shrink-0',
              isDone && 'bg-emerald-500/15',
              isError && 'bg-red-500/15',
              isRunning && 'bg-indigo-500/15',
            )}
          >
            {isRunning && <Loader2 className="w-4 h-4 text-indigo-500 animate-spin" />}
            {isDone && <CheckCircle className="w-4 h-4 text-emerald-500" />}
            {isError && <AlertCircle className="w-4 h-4 text-red-500" />}
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 min-w-0">
                <Users className="w-3 h-3 text-muted-foreground flex-shrink-0" />
                <p
                  className={cn(
                    'text-sm font-semibold truncate',
                    isDone && 'text-emerald-600 dark:text-emerald-400',
                    isError && 'text-red-600 dark:text-red-400',
                    isRunning && 'text-indigo-600 dark:text-indigo-400',
                  )}
                >
                  {teamName}
                </p>
              </div>
              <button
                onClick={() => {
                  if (isRunning) stopExecution();
                  clearActiveTeamTask();
                }}
                className="p-0.5 rounded-md hover:bg-accent text-muted-foreground transition-colors flex-shrink-0"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5 flex items-center gap-1">
              {isRunning && <Clock className="w-3 h-3" />}
              {isRunning
                ? hasRealOutput
                  ? `输出中... (${elapsed}s)`
                  : `执行中... (${serverElapsed ?? elapsed}s)`
                : isDone
                  ? '执行完成'
                  : '执行失败'}
            </p>
          </div>
        </div>

        <p className="text-xs text-muted-foreground bg-accent/50 rounded-lg px-3 py-2 mb-3 line-clamp-2">
          {task}
        </p>

        {/* Thinking heartbeat — subtle status, not output */}
        {isThinkingPlaceholder && (
          <div className="flex items-center gap-2 text-xs text-muted-foreground mb-2">
            <span className="flex gap-0.5">
              <span className="w-1 h-1 rounded-full bg-indigo-400 animate-bounce [animation-delay:0ms]" />
              <span className="w-1 h-1 rounded-full bg-indigo-400 animate-bounce [animation-delay:150ms]" />
              <span className="w-1 h-1 rounded-full bg-indigo-400 animate-bounce [animation-delay:300ms]" />
            </span>
            <span>AI 正在处理...</span>
          </div>
        )}

        {/* Slow warning — shown when backend hasn't responded for 60s */}
        {showSlowWarning && (
          <p className="text-xs text-amber-500/80 mb-2">已等待 {elapsed}s，任务可能需要较长时间</p>
        )}

        {hasRealOutput && (
          <div
            ref={outputRef}
            className="rounded-xl bg-indigo-500/5 border border-indigo-500/20 px-3 py-2 max-h-28 overflow-y-auto"
          >
            <p className="text-xs text-foreground/80 whitespace-pre-wrap font-mono leading-relaxed">
              {partialOutput}
            </p>
          </div>
        )}
        {isDone && result && (
          <div className="rounded-xl bg-emerald-500/5 border border-emerald-500/20 px-3 py-2 max-h-28 overflow-y-auto">
            <p className="text-xs text-foreground whitespace-pre-wrap">{result}</p>
          </div>
        )}
        {isError && error && (
          <div className="rounded-xl bg-red-500/5 border border-red-500/20 px-3 py-2">
            <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
          </div>
        )}

        {!isRunning && (
          <button
            onClick={clearActiveTeamTask}
            className="mt-3 w-full py-1.5 rounded-xl text-xs font-medium bg-accent hover:bg-accent/80 transition-colors"
          >
            关闭
          </button>
        )}
      </div>
    </div>
  );
}

/** 用于 GlobalOpsOverlay 堆叠容器内（无 fixed） */
export function TeamTaskCardInline() {
  return <TaskCardContent />;
}

/** 独立使用时带 fixed 定位 */
export function TeamTaskCard() {
  const activeTeamTask = useTeamStore((s) => s.activeTeamTask);
  if (!activeTeamTask) return null;
  return (
    <div className="fixed bottom-4 right-4 z-50">
      <TaskCardContent />
    </div>
  );
}
