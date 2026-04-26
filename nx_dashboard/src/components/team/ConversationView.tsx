import { useState, useEffect, useRef, useCallback, memo } from 'react';
import { X, Send, Bot, User, MessageCircle, Square, Terminal as TerminalIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, Message } from '@/stores/teamStore';
import { useAgentExecution } from '@/hooks/useAgentExecution';
import { TerminalPanel } from './TerminalPanel';
import { EmbeddedTerminalPreview } from './EmbeddedTerminalPreview';

// ── 独立输入组件，隔离重渲染 ──────────────────────────────
interface ChatInputProps {
  isActive: boolean;
  onSend: (text: string) => void;
  onCancel: () => void;
}

const ChatInput = memo(function ChatInput({ isActive, onSend, onCancel }: ChatInputProps) {
  const [value, setValue] = useState('');
  const isComposingRef = useRef(false);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (isComposingRef.current) return;
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (value.trim() && !isActive) {
        onSend(value.trim());
        setValue('');
      }
    }
  };

  const handleClick = () => {
    if (isActive) {
      onCancel();
    } else if (value.trim()) {
      onSend(value.trim());
      setValue('');
    }
  };

  return (
    <div className="p-4 border-t border-border/50">
      <div className="flex gap-2">
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onCompositionStart={() => { isComposingRef.current = true; }}
          onCompositionEnd={() => { isComposingRef.current = false; }}
          placeholder={isActive ? "等待响应..." : "输入消息..."}
          className="input-field flex-1 resize-none"
          rows={1}
          disabled={isActive}
        />
        <button
          onClick={handleClick}
          disabled={!isActive && !value.trim()}
          className={cn(
            'btn-primary px-4',
            (!isActive && !value.trim()) ? 'opacity-50 cursor-not-allowed' : ''
          )}
        >
          {isActive ? (
            <Square className="w-4 h-4" />
          ) : (
            <Send className="w-4 h-4" />
          )}
        </button>
      </div>
    </div>
  );
});

// ── 消息气泡，避免列表整体重渲染 ─────────────────────────
const MessageBubble = memo(function MessageBubble({ message }: { message: Message }) {
  return (
    <div
      className={cn(
        'flex gap-3',
        message.role === 'user' ? 'justify-end' : 'justify-start'
      )}
    >
      {message.role === 'assistant' && (
        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-emerald-500 to-green-500 flex items-center justify-center flex-shrink-0">
          <Bot className="w-4 h-4 text-white" />
        </div>
      )}
      <div
        className={cn(
          'max-w-[80%] rounded-2xl px-4 py-2.5',
          message.role === 'user'
            ? 'bg-gradient-to-r from-indigo-500 to-purple-500 text-white'
            : 'bg-muted'
        )}
      >
        <p className="text-sm whitespace-pre-wrap">{message.content}</p>
        {message.created_at && (
          <p className={cn(
            'text-xs mt-1',
            message.role === 'user' ? 'text-white/70' : 'text-muted-foreground'
          )}>
            {new Date(message.created_at).toLocaleTimeString('zh-CN', {
              hour: '2-digit',
              minute: '2-digit'
            })}
          </p>
        )}
      </div>
      {message.role === 'user' && (
        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center flex-shrink-0">
          <User className="w-4 h-4 text-white" />
        </div>
      )}
    </div>
  );
});

// ── 主组件 ──────────────────────────────────────────────
interface ConversationViewProps {
  teamId: string;
  onClose: () => void;
}

export function ConversationView({ teamId, onClose }: ConversationViewProps) {
  const fetchMessages = useTeamStore((s) => s.fetchMessages);
  const storeMessages = useTeamStore((s) => s.messages);
  const teamMonitorMode = useTeamStore((s) => s.teamMonitorMode);
  const isMonitorMode = teamMonitorMode[teamId] ?? false;
  const roles = useTeamStore((s) => s.roles[teamId] ?? []);
  const fetchRoles = useTeamStore((s) => s.fetchRoles);
  const [activeTab, setActiveTab] = useState<'chat' | 'terminal'>('chat');
  const [localMessages, setLocalMessages] = useState<Message[]>([]);
  const [lastCliOutput, setLastCliOutput] = useState<string>('');
  const [showLastOutput, setShowLastOutput] = useState(false);
  const [pendingConfirmTask, setPendingConfirmTask] = useState<string | null>(null);
  const messagesScrollRef = useRef<HTMLDivElement>(null);
  const executingRef = useRef(false);

  // Async agent execution with WS progress tracking
  const agentExec = useAgentExecution();
  const isActive = agentExec.status === 'started' || agentExec.status === 'thinking';

  // 后端 PTY dispatch 返回 session_id 时，同步到 terminalSessions store
  // 这样 RoleTerminalTab 切换到该角色时能自动连上已有 session
  const setTerminalSession = useTeamStore((s) => s.setTerminalSession);
  useEffect(() => {
    if (agentExec.activeRoleId && agentExec.activeSessionId) {
      setTerminalSession(teamId, agentExec.activeRoleId, agentExec.activeSessionId);
    }
  }, [teamId, agentExec.activeRoleId, agentExec.activeSessionId, setTerminalSession]);

  useEffect(() => {
    const serverMessages = storeMessages.filter(m => m.team_id === teamId);
    setLocalMessages(prev => {
      const tempMessages = prev.filter(m => m.id.startsWith('temp-'));
      if (tempMessages.length === 0) return serverMessages;
      // 去重：如果服务端消息中已包含相同内容的消息，移除对应临时消息
      const serverUserContents = new Set(serverMessages.filter(m => m.role === 'user').map(m => m.content));
      const serverAssistantContents = new Set(serverMessages.filter(m => m.role === 'assistant').map(m => m.content));
      const keptTemp = tempMessages.filter(m => {
        if (m.role === 'assistant') return !serverAssistantContents.has(m.content);
        if (m.role === 'user') return !serverUserContents.has(m.content);
        return true;
      });
      return [...serverMessages, ...keptTemp];
    });
  }, [storeMessages, teamId]);

  useEffect(() => {
    if (agentExec.status === 'completed') {
      // 立即用 result 渲染临时 AI 回复气泡，不等 fetchMessages
      if (agentExec.result) {
        const tempAssistantMsg: Message = {
          id: `temp-assistant-${Date.now()}`,
          team_id: teamId,
          role: 'assistant',
          message_type: 'Assistant',
          content: agentExec.result,
          created_at: new Date().toISOString(),
        };
        setLocalMessages(prev => [...prev, tempAssistantMsg]);
      }
      if (agentExec.partialOutput) {
        setLastCliOutput(agentExec.partialOutput);
        setShowLastOutput(true);
      }
      // fetchMessages 回来后会覆盖 localMessages，临时气泡自然被真实消息替换
      fetchMessages(teamId);
      executingRef.current = false;
      const timer = setTimeout(() => agentExec.reset(), 500);
      return () => clearTimeout(timer);
    }
    if (agentExec.status === 'failed') {
      if (agentExec.partialOutput) {
        setLastCliOutput(agentExec.partialOutput);
        setShowLastOutput(true);
      }
      fetchMessages(teamId);
      executingRef.current = false;
      const timer = setTimeout(() => agentExec.reset(), 500);
      return () => clearTimeout(timer);
    }
  }, [agentExec.status, teamId, fetchMessages]);

  useEffect(() => {
    fetchMessages(teamId);
    fetchRoles(teamId);
  }, [teamId]);

  // 兜底轮询：isActive 期间每 10s 拉一次消息，防止 WS Completed 事件丢失导致消息不更新
  useEffect(() => {
    if (!isActive) return;
    let aborted = false;
    let timer: ReturnType<typeof setTimeout>;
    const abortController = new AbortController();
    const poll = async () => {
      if (aborted) return;
      try {
        await fetchMessages(teamId);
      } catch {
        // AbortController cancellation — ignore
      }
      if (!aborted) timer = setTimeout(poll, 10000);
    };
    poll();
    return () => { aborted = true; clearTimeout(timer); abortController.abort(); };
  }, [isActive, teamId, fetchMessages]);

  // 冻结保护：isActive 超过 5 分钟自动重置，防止 WS 丢失事件导致 UI 永久卡死
  useEffect(() => {
    if (!isActive) return;
    const timeout = setTimeout(() => {
      agentExec.reset();
    }, 300_000);
    return () => clearTimeout(timeout);
  }, [isActive, agentExec.reset]);

  useEffect(() => {
    const el = messagesScrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [localMessages]);

  const handleSend = useCallback((text: string) => {
    if (!text || isActive || executingRef.current) return;

    // 监控模式 ON：先显示确认弹框，不直接执行
    if (isMonitorMode) {
      setPendingConfirmTask(text);
      return;
    }

    executingRef.current = true;

    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      team_id: teamId,
      role: 'user',
      message_type: 'User',
      content: text,
      created_at: new Date().toISOString(),
    };

    setLocalMessages(prev => [...prev, userMessage]);
    agentExec.execute(teamId, text).finally(() => {
      executingRef.current = false;
    });
  }, [teamId, isActive, agentExec, isMonitorMode]);

  const handleConfirmTask = useCallback(() => {
    if (!pendingConfirmTask || executingRef.current) return;
    executingRef.current = true;
    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      team_id: teamId,
      role: 'user',
      message_type: 'User',
      content: pendingConfirmTask,
      created_at: new Date().toISOString(),
    };
    setLocalMessages(prev => [...prev, userMessage]);
    agentExec.execute(teamId, pendingConfirmTask, true).finally(() => {
      executingRef.current = false;
    });
    setPendingConfirmTask(null);
  }, [teamId, pendingConfirmTask, agentExec]);

  const handleCancelTask = useCallback(() => {
    setPendingConfirmTask(null);
  }, []);

  const handleCancel = useCallback(() => {
    setPendingConfirmTask(null);
    agentExec.cancel();
  }, [agentExec]);

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-black/40" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-emerald-500/5 to-green-500/5">
          <div className="flex items-center gap-1">
            <button
              onClick={() => setActiveTab('chat')}
              className={cn(
                'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
                activeTab === 'chat' ? 'bg-emerald-500/20 text-emerald-400' : 'text-muted-foreground hover:bg-accent'
              )}
            >
              <MessageCircle className="w-4 h-4" />
              对话
            </button>
            <button
              onClick={() => setActiveTab('terminal')}
              className={cn(
                'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
                activeTab === 'terminal' ? 'bg-green-500/20 text-green-400' : 'text-muted-foreground hover:bg-accent'
              )}
            >
              <TerminalIcon className="w-4 h-4" />
              终端
            </button>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* 终端 Tab（始终挂载，切换时用 hidden 隐藏，避免断连） */}
        <div className={cn('flex-1 overflow-hidden', activeTab !== 'terminal' && 'hidden')}>
          <TerminalPanel teamId={teamId} roles={roles} visible={activeTab === 'terminal'} activeRoleId={agentExec.activeRoleId} />
        </div>

        {/* 对话 Tab */}
        {activeTab === 'chat' && <>
        {/* Messages */}
        <div ref={messagesScrollRef} className="flex-1 overflow-y-auto p-4 space-y-4">
          {localMessages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
              <Bot className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">开始与团队对话</p>
              <p className="text-sm mt-1">发送消息来测试团队协作</p>
            </div>
          ) : (
            localMessages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))
          )}
          {showLastOutput && lastCliOutput && !isActive && (
            <div className="flex gap-3 justify-start">
              <div className="w-8 h-8 rounded-full bg-[#252526] flex items-center justify-center flex-shrink-0">
                <TerminalIcon className="w-4 h-4 text-green-400" />
              </div>
              <div className="flex-1 max-w-[85%]">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs text-muted-foreground">执行过程</span>
                  <button
                    onClick={() => setShowLastOutput(false)}
                    className="text-xs text-muted-foreground hover:text-foreground"
                  >
                    收起
                  </button>
                </div>
                <div className="bg-[#1a1a1a] rounded-xl px-3 py-2 text-xs text-green-400 max-h-64 overflow-y-auto border border-white/5">
                  <pre className="whitespace-pre-wrap font-mono leading-relaxed">{lastCliOutput}</pre>
                </div>
              </div>
            </div>
          )}
          {pendingConfirmTask && (
            <div className="flex gap-3 justify-start">
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-amber-500 to-orange-500 flex items-center justify-center flex-shrink-0">
                <Bot className="w-4 h-4 text-white" />
              </div>
              <div className="flex-1 max-w-[85%] bg-amber-500/10 rounded-2xl px-4 py-3 border border-amber-500/20">
                <p className="text-sm text-amber-300 font-semibold mb-1">⚠ 监控模式 — 确认执行此任务？</p>
                <p className="text-sm text-amber-100/80 mb-3 break-all">{pendingConfirmTask}</p>
                <div className="flex gap-2">
                  <button
                    onClick={handleConfirmTask}
                    className="px-4 py-1.5 text-sm bg-emerald-600 hover:bg-emerald-500 text-white rounded transition-colors font-medium"
                  >
                    允许
                  </button>
                  <button
                    onClick={handleCancelTask}
                    className="px-4 py-1.5 text-sm bg-[#252526] hover:bg-red-500/20 text-red-400 border border-red-500/30 rounded transition-colors"
                  >
                    拒绝
                  </button>
                </div>
              </div>
            </div>
          )}
          {agentExec.status === 'confirmation' && agentExec.confirmationQuestion && (
            <div className="flex gap-3 justify-start">
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-amber-500 to-orange-500 flex items-center justify-center flex-shrink-0">
                <Bot className="w-4 h-4 text-white" />
              </div>
              <div className="flex-1 max-w-[85%] bg-amber-500/10 rounded-2xl px-4 py-3 border border-amber-500/20">
                <p className="text-sm text-amber-200 font-medium mb-3">
                  {agentExec.confirmationQuestion}
                </p>
                <div className="flex flex-wrap gap-2">
                  {agentExec.confirmationOptions.map((option) => (
                    <button
                      key={option}
                      onClick={() => agentExec.sendConfirmation(option)}
                      className="px-3 py-1.5 text-sm bg-[#252526] hover:bg-amber-500/30 text-amber-200 border border-amber-500/30 rounded transition-colors"
                    >
                      {option}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}
          {agentExec.status === 'failed' && agentExec.error && (
            <div className="flex gap-3 justify-start">
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-red-500 to-orange-500 flex items-center justify-center flex-shrink-0">
                <Bot className="w-4 h-4 text-white" />
              </div>
              <div className="bg-red-500/10 rounded-2xl px-4 py-2.5">
                <p className="text-sm text-red-400">{agentExec.error}</p>
              </div>
            </div>
          )}
        </div>

        {/* 执行中：内嵌终端预览 */}
        {isActive && (
          <div className="px-4 pb-2">
            <EmbeddedTerminalPreview
              output={agentExec.partialOutput || ''}
              elapsedSecs={agentExec.elapsedSecs || 0}
              progress={agentExec.progress}
              onViewTerminal={() => setActiveTab('terminal')}
            />
          </div>
        )}

        {/* Input — 独立组件，打字不触发消息列表/终端重渲染 */}
        <ChatInput isActive={isActive} onSend={handleSend} onCancel={handleCancel} />
        </>}
      </div>
    </div>
  );
}
