import { Play, Users, Zap, FastForward } from 'lucide-react';
import { cn } from '@/lib/utils';
import { GroupSessionDetail } from '@/stores/groupChatStore';
import { getStatusBadge, getStrategyLabel } from './utils';

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
}: SessionHeaderProps) {
  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="text-xl font-bold">{currentSession.name}</h2>
            {getStatusBadge(currentSession.status)}
          </div>
          <p className="text-muted-foreground mt-1">{currentSession.topic}</p>
          <div className="flex items-center gap-4 mt-3 text-sm">
            <span className="flex items-center gap-1">
              <Users className="w-4 h-4" />
              {getStrategyLabel(currentSession.speaking_strategy)}
            </span>
            <span className="flex items-center gap-1">
              <Zap className="w-4 h-4" />
              {currentSession.current_turn}/{currentSession.max_turns} 回合
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {currentSession.status === 'pending' && (
            <button onClick={onStartDiscussion} className="btn btn-primary">
              <Play className="w-4 h-4 mr-1" />
              开始讨论
            </button>
          )}
          {currentSession.status === 'active' && (
            <>
              <button
                onClick={onToggleAutoMode}
                className={cn('btn', autoMode ? 'btn-primary' : 'btn-outline')}
                disabled={isRoundRunning}
              >
                <Zap className="w-4 h-4 mr-1" />
                {autoMode ? '自动模式 ON' : '自动模式'}
              </button>
              <button
                onClick={onExecuteRound}
                disabled={isRoundRunning || isAgentActive || !currentSession.participants?.length}
                className="btn btn-outline flex items-center gap-1"
                title="并行执行所有参与者的本轮发言（速度约提升 N 倍）"
              >
                <FastForward className="w-4 h-4" />
                全员并行
              </button>
              <button onClick={onConcludeDiscussion} className="btn btn-outline">
                结束讨论
              </button>
            </>
          )}
        </div>
      </div>

      {/* Participants */}
      {currentSession.participants && currentSession.participants.length > 0 && (
        <div className="mt-4 pt-4 border-t">
          <h3 className="text-sm font-medium mb-2">参与者</h3>
          <div className="flex flex-wrap gap-2">
            {currentSession.participants.map((p) => (
              <div
                key={p.role_id}
                className={cn(
                  'px-3 py-1 rounded-full bg-secondary text-sm flex items-center gap-2',
                  nextSpeaker?.role_id === p.role_id && 'ring-2 ring-primary',
                )}
              >
                <span>{p.role_name}</span>
                {nextSpeaker?.role_id === p.role_id && (
                  <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                )}
                <span className="text-xs text-muted-foreground">{p.message_count}条</span>
                {currentSession.status === 'active' && !executingRole && !isAgentActive && (
                  <button
                    onClick={() => onExecuteRoleTurn(p.role_id)}
                    className="ml-1 p-0.5 hover:bg-primary/20 rounded"
                    disabled={executingRole !== null || isAgentActive}
                  >
                    <Play className="w-3 h-3" />
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
