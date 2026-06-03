import { MessageSquare, Clock, Zap, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { GroupSession } from '@/stores/groupChatStore';
import { getStatusBadge } from './utils';
import { DiscussionEmptyState } from './DiscussionEmptyState';

export interface SessionsListProps {
  sessions: GroupSession[];
  selectedSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onDeleteSession: (session: GroupSession) => void;
  embedded?: boolean;
}

export function SessionsList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onDeleteSession,
  embedded = false,
}: SessionsListProps) {
  return (
    <aside
      className={cn(
        'flex flex-col min-h-0 bg-muted/10',
        embedded ? 'w-72 shrink-0 border-r border-border/60' : 'col-span-4 space-y-4',
      )}
    >
      <div
        className={cn(
          'flex flex-col min-h-0 overflow-hidden',
          embedded ? 'h-full' : 'rounded-xl border border-border/60 bg-card shadow-sm',
        )}
      >
        <div className="flex shrink-0 items-center justify-between gap-2 border-b border-border/60 px-4 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold">
            <MessageSquare className="h-4 w-4 text-muted-foreground" />
            讨论会话
            <span className="rounded-full bg-secondary px-1.5 py-0.5 text-[11px] font-normal text-muted-foreground tabular-nums">
              {sessions.length}
            </span>
          </h2>
        </div>

        <div
          className={cn(
            'flex-1 overflow-y-auto p-2',
            embedded ? 'max-h-none' : 'max-h-[600px]',
          )}
        >
          {sessions.length === 0 ? (
            <DiscussionEmptyState
              compact
              icon={<MessageSquare className="h-5 w-5" />}
              title="暂无讨论会话"
              description="点击上方「新建讨论」开始"
            />
          ) : (
            <ul className="space-y-1">
              {sessions.map((session) => {
                const active = selectedSessionId === session.id;
                return (
                  <li key={session.id}>
                    <div
                      role="button"
                      tabIndex={0}
                      onClick={() => onSelectSession(session.id)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          onSelectSession(session.id);
                        }
                      }}
                      className={cn(
                        'group relative cursor-pointer rounded-lg border px-3 py-2.5 transition-colors',
                        active
                          ? 'border-primary/30 bg-primary/5 shadow-sm'
                          : 'border-transparent hover:border-border/60 hover:bg-accent/50',
                      )}
                    >
                      {active && (
                        <span className="absolute bottom-2 left-0 top-2 w-0.5 rounded-full bg-primary" />
                      )}
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span
                              className={cn(
                                'truncate text-sm font-medium',
                                active ? 'text-primary' : 'text-foreground',
                              )}
                            >
                              {session.name}
                            </span>
                            {getStatusBadge(session.status)}
                          </div>
                          <p className="mt-1 truncate text-xs text-muted-foreground">{session.topic}</p>
                          <div className="mt-2 flex items-center gap-3 text-[11px] text-muted-foreground">
                            <span className="flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {new Date(session.created_at).toLocaleDateString()}
                            </span>
                            <span className="flex items-center gap-1 tabular-nums">
                              <Zap className="h-3 w-3" />
                              {session.current_turn}/{session.max_turns}
                            </span>
                          </div>
                        </div>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onDeleteSession(session);
                          }}
                          className="rounded-md p-1.5 text-muted-foreground opacity-0 transition-all hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
                          title="删除会话"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </aside>
  );
}
