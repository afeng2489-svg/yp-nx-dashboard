import { MessageSquare, Clock, Zap, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { GroupSession } from '@/stores/groupChatStore';
import { getStatusBadge } from './utils';

export interface SessionsListProps {
  sessions: GroupSession[];
  selectedSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onDeleteSession: (session: GroupSession) => void;
}

export function SessionsList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onDeleteSession,
}: SessionsListProps) {
  return (
    <div className="col-span-4 space-y-4">
      <div className="bg-card rounded-lg border">
        <div className="p-4 border-b">
          <h2 className="font-semibold flex items-center gap-2">
            <MessageSquare className="w-4 h-4" />
            讨论会话
          </h2>
        </div>
        <div className="divide-y max-h-[600px] overflow-y-auto">
          {sessions.length === 0 ? (
            <div className="p-8 text-center text-muted-foreground">暂无讨论会话</div>
          ) : (
            sessions.map((session) => (
              <div
                key={session.id}
                onClick={() => onSelectSession(session.id)}
                className={cn(
                  'p-4 cursor-pointer hover:bg-accent transition-colors',
                  selectedSessionId === session.id && 'bg-accent',
                )}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium truncate">{session.name}</span>
                      {getStatusBadge(session.status)}
                    </div>
                    <p className="text-sm text-muted-foreground truncate mt-1">{session.topic}</p>
                    <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                      <span className="flex items-center gap-1">
                        <Clock className="w-3 h-3" />
                        {new Date(session.created_at).toLocaleDateString()}
                      </span>
                      <span className="flex items-center gap-1">
                        <Zap className="w-3 h-3" />
                        {session.current_turn}/{session.max_turns}
                      </span>
                    </div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteSession(session);
                    }}
                    className="p-1.5 hover:bg-destructive/20 rounded text-destructive"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
