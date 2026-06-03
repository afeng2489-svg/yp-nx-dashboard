import { Play, Users, Zap, FastForward } from 'lucide-react';
import { cn } from '@/lib/utils';
import { GroupSessionDetail } from '@/stores/groupChatStore';
import { getStatusBadge, getStrategyLabel, roleVisual } from './utils';

export interface SessionHeaderProps {
  currentSession: GroupSessionDetail;
  nextSpeaker: { role_id: string; role_name: string } | null;
  autoMode: boolean;
  isRoundRunning: boolean;
  isAgentActive: boolean;
  executingRole: string | null;
  onStartDiscussion: () => void;
  onToggleAutoMode: () => void;
  onExecuteRound: () => void;
  onConcludeDiscussion: () => void;
  onExecuteRoleTurn: (roleId: string) => void;
  compact?: boolean;
}

export function SessionHeader({
  currentSession,
  nextSpeaker,
  autoMode,
  isRoundRunning,
  isAgentActive,
  executingRole,
  onStartDiscussion,
  onToggleAutoMode,
  onExecuteRound,
  onConcludeDiscussion,
  onExecuteRoleTurn,
  compact = false,
}: SessionHeaderProps) {
  const turnPct = currentSession.max_turns
    ? Math.min(100, (currentSession.current_turn / currentSession.max_turns) * 100)
    : 0;

  return (
    <div
      className={cn(
        'rounded-xl border border-border/60 bg-card shadow-sm',
        compact ? 'p-4' : 'p-5',
      )}
    >
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2 sm:gap-3">
            <h2 className={cn('truncate font-bold', compact ? 'text-lg' : 'text-xl')}>
              {currentSession.name}
            </h2>
            {getStatusBadge(currentSession.status)}
          </div>
          <p className="mt-1 text-sm text-muted-foreground">{currentSession.topic}</p>
          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Users className="h-4 w-4 shrink-0" />
              {getStrategyLabel(currentSession.speaking_strategy)}
            </span>
            <span className="flex items-center gap-2">
              <Zap className="h-4 w-4 shrink-0" />
              <span className="tabular-nums">
                {currentSession.current_turn}/{currentSession.max_turns} 回合
              </span>
              <span className="hidden h-1.5 w-20 overflow-hidden rounded-full bg-secondary sm:inline-block">
                <span
                  className="block h-full rounded-full bg-primary transition-all"
                  style={{ width: `${turnPct}%` }}
                />
              </span>
            </span>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 shrink-0">
          {currentSession.status === 'pending' && (
            <button type="button" onClick={onStartDiscussion} className="btn btn-primary h-9 px-4 text-sm">
              <Play className="h-4 w-4" />
              开始讨论
            </button>
          )}
          {currentSession.status === 'active' && (
            <>
              <button
                type="button"
                onClick={onToggleAutoMode}
                className={cn('btn h-9 px-3 text-sm', autoMode ? 'btn-primary' : 'btn-ghost border border-border')}
                disabled={isRoundRunning}
              >
                <Zap className="h-4 w-4" />
                {autoMode ? '自动 ON' : '自动模式'}
              </button>
              <button
                type="button"
                onClick={onExecuteRound}
                disabled={isRoundRunning || isAgentActive || !currentSession.participants?.length}
                className="btn btn-ghost h-9 border border-border px-3 text-sm"
                title="并行执行所有参与者的本轮发言"
              >
                <FastForward className="h-4 w-4" />
                全员并行
              </button>
              <button
                type="button"
                onClick={onConcludeDiscussion}
                className="btn btn-ghost h-9 border border-border px-3 text-sm"
              >
                结束讨论
              </button>
            </>
          )}
        </div>
      </div>

      {/* Participants */}
      {currentSession.participants && currentSession.participants.length > 0 && (
        <div className="mt-4 pt-4 border-t">
          <div className="flex items-center justify-between mb-2.5">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              参与者 ({currentSession.participants.length})
            </h3>
            {nextSpeaker && (
              <span className="text-xs text-muted-foreground">
                下一位：<span className="font-medium text-foreground">{nextSpeaker.role_name}</span>
              </span>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            {currentSession.participants.map((p) => {
              const v = roleVisual(p.role_name);
              const isNext = nextSpeaker?.role_id === p.role_id;
              const canRun =
                currentSession.status === 'active' && !executingRole && !isAgentActive;
              return (
                <div
                  key={p.role_id}
                  className={cn(
                    'group flex items-center gap-2 pl-1.5 pr-2.5 py-1 rounded-full border text-sm transition-all',
                    isNext
                      ? cn('ring-2 ring-offset-1 ring-offset-card', v.ring, v.soft)
                      : 'bg-secondary/60 border-transparent hover:border-border',
                  )}
                  title={isNext ? '下一位发言' : undefined}
                >
                  <span
                    className={cn(
                      'w-6 h-6 rounded-full flex items-center justify-center text-white text-[11px] font-semibold flex-shrink-0',
                      v.avatar,
                    )}
                  >
                    {p.role_name.charAt(0).toUpperCase()}
                  </span>
                  <span className={cn('font-medium', isNext && v.text)}>{p.role_name}</span>
                  {isNext && (
                    <span className={cn('w-1.5 h-1.5 rounded-full animate-pulse', v.dot)} />
                  )}
                  <span className="text-xs text-muted-foreground tabular-nums">
                    {p.message_count}
                  </span>
                  {canRun && (
                    <button
                      onClick={() => onExecuteRoleTurn(p.role_id)}
                      className="ml-0.5 p-1 rounded-full hover:bg-primary hover:text-primary-foreground text-muted-foreground transition-colors"
                      disabled={executingRole !== null || isAgentActive}
                      title={`让 ${p.role_name} 发言`}
                    >
                      <Play className="w-3 h-3" />
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
