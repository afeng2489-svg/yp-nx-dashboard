import { FastForward, CheckCircle, Loader2, AlertCircle, Clock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ParallelBotState } from './hooks/useParallelRound';

export interface ParallelRoundProgressProps {
  bots: ParallelBotState[];
}

export function ParallelRoundProgress({ bots }: ParallelRoundProgressProps) {
  if (bots.length === 0) return null;

  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-semibold text-sm flex items-center gap-2">
          <FastForward className="w-4 h-4 text-primary" />
          全员并行执行中
        </h3>
        <span className="text-xs text-muted-foreground">
          {bots.filter((b) => b.status === 'done').length} / {bots.length} 完成
        </span>
      </div>
      <div className="grid grid-cols-2 gap-2">
        {bots.map((bot) => (
          <div
            key={bot.execution_id}
            className={cn(
              'flex items-center gap-2 px-3 py-2 rounded-md text-sm border',
              bot.status === 'done' && 'bg-green-500/10 border-green-500/30',
              bot.status === 'thinking' && 'bg-primary/10 border-primary/30',
              bot.status === 'failed' && 'bg-destructive/10 border-destructive/30',
              bot.status === 'pending' && 'bg-secondary/50 border-border',
            )}
          >
            {bot.status === 'done' && (
              <CheckCircle className="w-3.5 h-3.5 text-green-500 flex-shrink-0" />
            )}
            {bot.status === 'thinking' && (
              <Loader2 className="w-3.5 h-3.5 text-primary animate-spin flex-shrink-0" />
            )}
            {bot.status === 'failed' && (
              <AlertCircle className="w-3.5 h-3.5 text-destructive flex-shrink-0" />
            )}
            {bot.status === 'pending' && (
              <Clock className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
            )}
            <span className="truncate">{bot.role_name ?? bot.role_id}</span>
            {bot.status === 'thinking' && bot.elapsed_secs > 0 && (
              <span className="ml-auto text-xs text-muted-foreground flex-shrink-0">
                {bot.elapsed_secs}s
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
