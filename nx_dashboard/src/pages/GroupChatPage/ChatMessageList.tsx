import { MessageSquare } from 'lucide-react';
import { GroupMessage, GroupSessionDetail } from '@/stores/groupChatStore';
import { AgentThinkingIndicator } from '@/components/team/AgentThinkingIndicator';
import { UseAgentExecutionReturn } from '@/hooks/useAgentExecution';

export interface ChatMessageListProps {
  messages: GroupMessage[];
  selectedSessionId: string;
  currentSession: GroupSessionDetail | null;
  executingRole: string | null;
  isAgentActive: boolean;
  agentExec: UseAgentExecutionReturn;
  onRefresh: () => void;
  onCancelExecution: () => void;
}

export function ChatMessageList({
  messages,
  currentSession,
  executingRole,
  isAgentActive,
  agentExec,
  onRefresh,
  onCancelExecution,
}: ChatMessageListProps) {
  return (
    <div className="bg-card rounded-lg border">
      <div className="p-4 border-b flex items-center justify-between">
        <h3 className="font-semibold flex items-center gap-2">
          <MessageSquare className="w-4 h-4" />
          讨论记录 ({messages.length})
        </h3>
        <button onClick={onRefresh} className="text-sm text-primary hover:underline">
          刷新
        </button>
      </div>
      <div className="max-h-[400px] overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="text-center text-muted-foreground py-8">暂无讨论记录</div>
        ) : (
          messages.map((msg) => (
            <div key={msg.id} className="flex gap-3">
              <div className="w-8 h-8 rounded-full bg-secondary flex items-center justify-center flex-shrink-0">
                <span className="text-xs font-medium">{msg.role_name.charAt(0).toUpperCase()}</span>
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm">{msg.role_name}</span>
                  <span className="text-xs text-muted-foreground">#{msg.turn_number}</span>
                  <span className="text-xs text-muted-foreground">
                    {new Date(msg.created_at).toLocaleTimeString()}
                  </span>
                </div>
                <p className="mt-1 text-sm whitespace-pre-wrap">{msg.content}</p>
              </div>
            </div>
          ))
        )}
      </div>
      {executingRole && isAgentActive && (
        <div className="p-4 border-t">
          <AgentThinkingIndicator
            agentRole={
              currentSession?.participants?.find((p) => p.role_id === executingRole)?.role_name
            }
            elapsedSecs={agentExec.elapsedSecs}
            onCancel={onCancelExecution}
            partialOutput={agentExec.partialOutput || undefined}
          />
        </div>
      )}
      {executingRole && agentExec.status === 'failed' && agentExec.error && (
        <div className="p-4 border-t text-sm text-red-400">执行失败: {agentExec.error}</div>
      )}
    </div>
  );
}
