import { useState } from 'react';
import { cn } from '@/lib/utils';
import { ConversationView } from '@/components/team/ConversationView';
import { GroupChatPage } from '@/pages/GroupChatPage';
import { isP5TeamChatUnifiedEnabled } from '@/data/factoryFeatureFlags';

type ChatMode = 'dispatch' | 'discuss';

export interface TeamChatUnifiedProps {
  teamId: string;
}

/** AF-UX-04b：派活 | 讨论 单页 */
export function TeamChatUnified({ teamId }: TeamChatUnifiedProps) {
  const [mode, setMode] = useState<ChatMode>('dispatch');
  const unified = isP5TeamChatUnifiedEnabled();

  if (!unified) {
    return (
      <div className="h-[min(560px,calc(100vh-16rem))] rounded-xl border border-border/50 overflow-hidden">
        <ConversationView teamId={teamId} embedded />
      </div>
    );
  }

  return (
    <div className="space-y-3 min-h-[480px]" data-testid="team-chat-unified">
      <div className="inline-flex rounded-lg border border-border p-0.5 bg-muted/30">
        {(
          [
            { id: 'dispatch' as const, label: '派活' },
            { id: 'discuss' as const, label: '讨论' },
          ] as const
        ).map((m) => (
          <button
            key={m.id}
            type="button"
            data-testid={`team-chat-mode-${m.id}`}
            className={cn(
              'px-4 py-1.5 text-sm rounded-md transition-colors',
              mode === m.id ? 'bg-background shadow-sm font-medium' : 'text-muted-foreground',
            )}
            onClick={() => setMode(m.id)}
          >
            {m.label}
          </button>
        ))}
      </div>

      {mode === 'dispatch' ? (
        <div className="h-[min(520px,calc(100vh-18rem))] rounded-xl border border-border/50 overflow-hidden">
          <ConversationView teamId={teamId} embedded />
        </div>
      ) : (
        <div className="min-h-[480px] rounded-xl border border-border/50 overflow-hidden">
          <GroupChatPage embedded teamId={teamId} />
        </div>
      )}
    </div>
  );
}
