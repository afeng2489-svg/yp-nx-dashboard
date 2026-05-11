import { useEffect, useMemo, useState } from 'react';
import { Bot, MessageCircle, RefreshCw, User, Hash } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, Message, Role } from '@/stores/teamStore';
import { MarkdownMessage } from '@/components/common/MarkdownMessage';

interface TelegramConversationsPanelProps {
  teamId: string;
}

interface ChatThread {
  chatId: string;
  roleId: string | null;
  roleName: string;
  messages: Message[];
  latestAt: number;
}

function isTelegramMessage(msg: Message): boolean {
  return msg.metadata?.source === 'telegram';
}

function groupByChat(messages: Message[], roles: Role[]): ChatThread[] {
  const byKey = new Map<string, ChatThread>();
  const roleById = new Map(roles.map((r) => [r.id, r]));

  for (const msg of messages) {
    if (!isTelegramMessage(msg)) continue;

    const chatId = msg.metadata?.chat_id ?? 'unknown';
    const roleId = msg.metadata?.role_id ?? msg.role_id ?? null;
    const key = `${chatId}::${roleId ?? ''}`;
    const ts = msg.created_at ? new Date(msg.created_at).getTime() : 0;

    const existing = byKey.get(key);
    if (existing) {
      existing.messages.push(msg);
      if (ts > existing.latestAt) existing.latestAt = ts;
    } else {
      byKey.set(key, {
        chatId,
        roleId,
        roleName: roleId ? (roleById.get(roleId)?.name ?? '未知角色') : '未指定',
        messages: [msg],
        latestAt: ts,
      });
    }
  }

  return Array.from(byKey.values()).sort((a, b) => b.latestAt - a.latestAt);
}

export function TelegramConversationsPanel({ teamId }: TelegramConversationsPanelProps) {
  const { messages, fetchMessages, roles, fetchRoles } = useTeamStore();
  const [loading, setLoading] = useState(false);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    try {
      await Promise.all([fetchMessages(teamId), fetchRoles(teamId)]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 10000);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [teamId]);

  const threads = useMemo(() => {
    const teamRoles = roles[teamId] ?? [];
    return groupByChat(messages, teamRoles);
  }, [messages, roles, teamId]);

  const currentThread = useMemo(() => {
    if (!selectedKey) return threads[0] ?? null;
    return (
      threads.find((t) => `${t.chatId}::${t.roleId ?? ''}` === selectedKey) ?? threads[0] ?? null
    );
  }, [threads, selectedKey]);

  if (threads.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-12 text-center text-muted-foreground">
        <MessageCircle className="w-12 h-12 mb-4 opacity-40" />
        <p className="font-medium">暂无 Telegram 对话记录</p>
        <p className="text-sm mt-1">启动 Bot 并在 TG 里发消息后这里会显示</p>
        <button
          onClick={refresh}
          disabled={loading}
          className="mt-4 flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg border hover:bg-accent transition-colors"
        >
          <RefreshCw className={cn('w-3.5 h-3.5', loading && 'animate-spin')} />
          刷新
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-[400px]">
      {/* Thread list */}
      <div className="w-48 flex-shrink-0 border-r border-border/50 overflow-y-auto">
        <div className="px-3 py-2 flex items-center justify-between border-b border-border/50 bg-muted/30">
          <span className="text-xs font-medium text-muted-foreground">会话 ({threads.length})</span>
          <button
            onClick={refresh}
            disabled={loading}
            className="p-1 rounded hover:bg-accent transition-colors"
            title="刷新"
          >
            <RefreshCw className={cn('w-3.5 h-3.5', loading && 'animate-spin')} />
          </button>
        </div>
        <div className="divide-y divide-border/30">
          {threads.map((t) => {
            const key = `${t.chatId}::${t.roleId ?? ''}`;
            const active =
              currentThread && key === `${currentThread.chatId}::${currentThread.roleId ?? ''}`;
            const lastMsg = t.messages[t.messages.length - 1];
            return (
              <button
                key={key}
                onClick={() => setSelectedKey(key)}
                className={cn(
                  'w-full text-left px-3 py-2 transition-colors',
                  active ? 'bg-blue-500/10' : 'hover:bg-accent/50',
                )}
              >
                <div className="flex items-center gap-1.5 text-sm font-medium truncate">
                  <Bot className="w-3.5 h-3.5 text-blue-500 flex-shrink-0" />
                  <span className="truncate">{t.roleName}</span>
                </div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground mt-0.5">
                  <Hash className="w-3 h-3" />
                  <span className="truncate">{t.chatId}</span>
                </div>
                {lastMsg && (
                  <p className="text-xs text-muted-foreground/80 mt-1 truncate">
                    {lastMsg.content}
                  </p>
                )}
              </button>
            );
          })}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {currentThread && (
          <>
            <div className="px-4 py-2 border-b border-border/50 bg-muted/20 flex items-center gap-2">
              <Bot className="w-4 h-4 text-blue-500" />
              <span className="text-sm font-medium">{currentThread.roleName}</span>
              <span className="text-xs text-muted-foreground">· Chat {currentThread.chatId}</span>
              <span className="text-xs text-muted-foreground ml-auto">
                {currentThread.messages.length} 条
              </span>
            </div>
            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {currentThread.messages.map((m) => {
                const isUser = m.role === 'user';
                return (
                  <div
                    key={m.id}
                    className={cn('flex gap-2', isUser ? 'justify-end' : 'justify-start')}
                  >
                    {!isUser && (
                      <div className="w-7 h-7 rounded-full bg-gradient-to-br from-emerald-500 to-green-500 flex items-center justify-center flex-shrink-0">
                        <Bot className="w-3.5 h-3.5 text-white" />
                      </div>
                    )}
                    <div
                      className={cn(
                        'max-w-[70%] rounded-xl px-3 py-2 text-sm',
                        isUser
                          ? 'bg-blue-500 text-white'
                          : 'bg-card border border-border/50 text-foreground',
                      )}
                    >
                      {isUser ? (
                        <p className="whitespace-pre-wrap break-words">{m.content}</p>
                      ) : (
                        <MarkdownMessage content={m.content} variant="assistant" />
                      )}
                      {m.created_at && (
                        <p
                          className={cn(
                            'text-[10px] mt-1',
                            isUser ? 'text-white/70' : 'text-muted-foreground',
                          )}
                        >
                          {new Date(m.created_at).toLocaleString('zh-CN')}
                        </p>
                      )}
                    </div>
                    {isUser && (
                      <div className="w-7 h-7 rounded-full bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center flex-shrink-0">
                        <User className="w-3.5 h-3.5 text-white" />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
