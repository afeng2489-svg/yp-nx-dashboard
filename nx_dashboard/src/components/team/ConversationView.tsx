import { useState, useEffect, useRef } from 'react';
import { X, Send, Bot, User, MessageCircle, Square, Terminal as TerminalIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, Message } from '@/stores/teamStore';
import { useClaudeStream } from '@/hooks/useClaudeStream';
import { useAgentExecution } from '@/hooks/useAgentExecution';
import { AgentThinkingIndicator } from './AgentThinkingIndicator';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

interface ConversationViewProps {
  teamId: string;
  onClose: () => void;
}

export function ConversationView({ teamId, onClose }: ConversationViewProps) {
  const { messages, fetchMessages } = useTeamStore();
  const [input, setInput] = useState('');
  const [localMessages, setLocalMessages] = useState<Message[]>([]);
  const [showStream, setShowStream] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const streamTerminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  // Async agent execution with WS progress tracking
  const agentExec = useAgentExecution();
  const isActive = agentExec.status === 'started' || agentExec.status === 'thinking';

  // Claude Stream hook for real-time output
  const { isConnected, isExecuting, output, execute, cancel, error: streamError } = useClaudeStream({
    onOutput: (line, isError) => {
      xtermRef.current?.write(
        isError
          ? `\x1b[31m${line}\r\n\x1b[0m`
          : `${line}\r\n`
      );
    },
    onComplete: (exitCode) => {
      xtermRef.current?.writeln(`\r\n\x1b[33m[进程退出，代码: ${exitCode}]\x1b[0m`);
    },
  });

  // Keep ref updated with latest messages
  useEffect(() => {
    setLocalMessages(messages.filter(m => m.team_id === teamId));
  }, [messages, teamId]);

  // When agent execution completes, refresh messages
  useEffect(() => {
    if (agentExec.status === 'completed' || agentExec.status === 'failed') {
      fetchMessages(teamId);
      // Reset after a short delay so the thinking indicator disappears
      const timer = setTimeout(() => agentExec.reset(), 500);
      return () => clearTimeout(timer);
    }
  }, [agentExec.status, teamId, fetchMessages]);

  useEffect(() => {
    fetchMessages(teamId);
  }, [teamId]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [localMessages]);
  useEffect(() => {
    if (!streamTerminalRef.current) return;

    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: 12,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#ffffff',
      },
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(streamTerminalRef.current);
    requestAnimationFrame(() => {
      fitAddon.fit();
    });

    xtermRef.current = terminal;
    fitAddonRef.current = fitAddon;

    terminal.writeln('\x1b[36m[Claude CLI Stream]\x1b[0m 连接中...');
    terminal.writeln('');

    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        // Ignore fit errors during rapid resize
      }
    });
    resizeObserver.observe(streamTerminalRef.current);

    return () => {
      resizeObserver.disconnect();
      terminal.dispose();
    };
  }, []);

  const handleSend = async () => {
    if (!input.trim() || isActive) return;

    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      team_id: teamId,
      role: 'user',
      message_type: 'User',
      content: input.trim(),
      created_at: new Date().toISOString(),
    };

    setLocalMessages(prev => [...prev, userMessage]);
    setInput('');

    // Non-blocking — returns immediately, WS tracks progress
    await agentExec.execute(teamId, userMessage.content);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-gradient-to-r from-black/20 to-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-emerald-500/5 to-green-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <MessageCircle className="w-5 h-5 text-emerald-500" />
            对话
          </h2>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowStream(!showStream)}
              className={cn(
                'p-2 rounded-lg transition-colors',
                showStream ? 'bg-emerald-500/20 text-emerald-500' : 'hover:bg-accent'
              )}
              title="CLI 流式输出"
            >
              <TerminalIcon className="w-5 h-5" />
            </button>
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {localMessages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
              <Bot className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">开始与团队对话</p>
              <p className="text-sm mt-1">发送消息来测试团队协作</p>
            </div>
          ) : (
            localMessages.map((message) => (
              <div
                key={message.id}
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
            ))
          )}
          {isActive && (
            <AgentThinkingIndicator
              agentRole={undefined}
              elapsedSecs={agentExec.elapsedSecs}
              onCancel={agentExec.cancel}
              partialOutput={agentExec.partialOutput || undefined}
            />
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
          <div ref={messagesEndRef} />
        </div>

        {/* Streaming Terminal Panel */}
        {showStream && (
          <div className="border-t border-border/50 bg-[#1e1e1e]">
            {/* Stream header */}
            <div className="flex items-center justify-between px-3 py-1.5 bg-[#252526]">
              <span className="text-xs text-gray-400 flex items-center gap-1">
                <TerminalIcon className="w-3 h-3" />
                CLI 流式输出
              </span>
              <div className="flex items-center gap-2">
                <span className={cn(
                  'w-2 h-2 rounded-full',
                  isConnected ? 'bg-green-500' : 'bg-yellow-500 animate-pulse'
                )} />
                <span className="text-xs text-gray-500">
                  {isExecuting ? '执行中...' : isConnected ? '就绪' : '连接中'}
                </span>
                {isExecuting && (
                  <button
                    onClick={cancel}
                    className="p-0.5 rounded hover:bg-red-500/20 text-red-400"
                  >
                    <Square className="w-3 h-3" />
                  </button>
                )}
              </div>
            </div>
            {/* Terminal */}
            <div ref={streamTerminalRef} className="h-[200px] px-2 py-1" />
            {/* Stream input */}
            <div className="flex gap-2 p-2 bg-[#252526] border-t border-[#3c3c3c]">
              <input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    if (isConnected && input.trim()) {
                      execute(input.trim());
                      setInput('');
                    }
                  }
                }}
                placeholder="输入命令，按 Enter 执行..."
                className="flex-1 px-3 py-1.5 text-sm bg-[#1e1e1e] border border-[#3c3c3c] rounded text-gray-300 focus:outline-none focus:border-[#007acc]"
                disabled={!isConnected || isExecuting}
              />
              <button
                onClick={() => {
                  if (input.trim()) {
                    execute(input.trim());
                    setInput('');
                  }
                }}
                disabled={!input.trim() || !isConnected || isExecuting}
                className="px-3 py-1.5 text-sm bg-[#007acc] hover:bg-[#007acc]/80 text-white rounded disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Send className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        )}

        {/* Input */}
        <div className="p-4 border-t border-border/50">
          <div className="flex gap-2">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={isActive ? "等待响应..." : "输入消息..."}
              className="input-field flex-1 resize-none"
              rows={1}
              disabled={isActive}
            />
            <button
              onClick={() => isActive ? agentExec.cancel() : handleSend()}
              disabled={!isActive && !input.trim()}
              className={cn(
                'btn-primary px-4',
                (!isActive && !input.trim()) ? 'opacity-50 cursor-not-allowed' : ''
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
      </div>
    </div>
  );
}
