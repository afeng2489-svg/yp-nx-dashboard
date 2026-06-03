import { MessageSquare, RefreshCw, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { GroupMessage, GroupSessionDetail } from '@/stores/groupChatStore';
import { AgentThinkingIndicator } from '@/components/team/AgentThinkingIndicator';
import { MarkdownMessage } from '@/components/common/MarkdownMessage';
import { UseAgentExecutionReturn } from '@/hooks/useAgentExecution';
import { roleVisual } from './utils';

export interface ChatMessageListProps {
  messages: GroupMessage[];
  selectedSessionId: string;
  currentSession: GroupSessionDetail | null;
  executingRole: string | null;
  isAgentActive: boolean;
  agentExec: UseAgentExecutionReturn;
  onRefresh: () => void;
  onCancelExecution: () => void;
  embedded?: boolean;
}

export function ChatMessageList({
  messages,
  currentSession,
  executingRole,
  isAgentActive,
  agentExec,
  onRefresh,
  onCancelExecution,
  embedded = false,
}: ChatMessageListProps) {
  return (
    <div
      className={cn(
        'overflow-hidden rounded-xl border border-border/60 bg-card shadow-sm',
        embedded && 'flex min-h-0 flex-1 flex-col',
      )}
    >
      <div className="flex shrink-0 items-center justify-between border-b border-border/60 bg-muted/20 px-4 py-3">
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <MessageSquare className="h-4 w-4 text-muted-foreground" />
          讨论记录
          <span className="rounded-full bg-secondary px-1.5 py-0.5 text-[11px] font-normal text-muted-foreground tabular-nums">
            {messages.length}
          </span>
        </h3>
        <button
          type="button"
          onClick={onRefresh}
          className="flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          <RefreshCw className="h-3.5 w-3.5" />
          刷新
        </button>
      </div>
      <div
        className={cn(
          'space-y-5 overflow-y-auto p-4',
          embedded ? 'min-h-[180px] flex-1' : 'max-h-[520px] min-h-[200px]',
        )}
      >
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-14 text-muted-foreground">
            <div className="w-12 h-12 rounded-full bg-secondary/60 flex items-center justify-center mb-3">
              <MessageSquare className="w-6 h-6 opacity-60" />
            </div>
            <p className="text-sm">还没有发言，开始讨论后这里会实时显示对话</p>
          </div>
        ) : (
          messages.map((msg) => {
            const v = roleVisual(msg.role_name);
            return (
              <div key={msg.id} className="flex gap-3 group">
                <div
                  className={cn(
                    'w-9 h-9 rounded-full flex items-center justify-center flex-shrink-0 text-white text-sm font-semibold shadow-sm ring-2 ring-background',
                    v.avatar,
                  )}
                >
                  {msg.role_name.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={cn('font-semibold text-sm', v.text)}>{msg.role_name}</span>
                    <span className="px-1.5 py-0.5 rounded bg-secondary text-[10px] text-muted-foreground">
                      第 {msg.turn_number} 轮
                    </span>
                    <span className="text-[11px] text-muted-foreground">
                      {new Date(msg.created_at).toLocaleTimeString('zh-CN', {
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </span>
                  </div>
                  <div className="rounded-2xl rounded-tl-sm bg-muted/60 border border-border/50 px-3.5 py-2.5">
                    <MarkdownMessage content={msg.content} variant="assistant" />
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
      {executingRole && isAgentActive && (
        <div className="p-4 border-t bg-muted/20">
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
        <div className="px-4 py-3 border-t bg-destructive/5 text-sm text-destructive flex items-center gap-2">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          执行失败: {agentExec.error}
        </div>
      )}
    </div>
  );
}
