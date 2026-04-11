import { useState, useEffect, useRef } from 'react';
import { X, Send, Loader2, Bot, User, MessageCircle, Square } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, Message } from '@/stores/teamStore';

interface ConversationViewProps {
  teamId: string;
  onClose: () => void;
}

export function ConversationView({ teamId, onClose }: ConversationViewProps) {
  const { messages, fetchMessages, executeTask } = useTeamStore();
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [localMessages, setLocalMessages] = useState<Message[]>([]);
  const [processing, setProcessing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const pollingIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const messagesRef = useRef<Message[]>(messages);

  // Keep ref updated with latest messages
  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  useEffect(() => {
    fetchMessages(teamId);
  }, [teamId]);

  useEffect(() => {
    setLocalMessages(messages.filter(m => m.team_id === teamId));
  }, [messages, teamId]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [localMessages]);

  // Cleanup polling on unmount
  useEffect(() => {
    return () => {
      if (pollingIntervalRef.current) {
        clearInterval(pollingIntervalRef.current);
      }
    };
  }, []);

  // Start polling for new messages when processing
  const startPolling = () => {
    setProcessing(true);
    let pollsRemaining = 60; // Poll for up to 60 seconds

    pollingIntervalRef.current = setInterval(async () => {
      await fetchMessages(teamId);
      pollsRemaining--;

      if (pollsRemaining <= 0) {
        stopPolling();
      }
    }, 1000);
  };

  // Stop polling
  const stopPolling = () => {
    if (pollingIntervalRef.current) {
      clearInterval(pollingIntervalRef.current);
      pollingIntervalRef.current = null;
    }
    setProcessing(false);
    setSending(false);
  };

  const handleSend = async () => {
    if (!input.trim() || sending) return;

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
    setSending(true);

    try {
      // Execute task - now returns immediately with background processing
      const response = await executeTask(teamId, userMessage.content);

      if (response.error) {
        // Show error immediately
        const errorMessage: Message = {
          id: `temp-${Date.now() + 1}`,
          team_id: teamId,
          role: 'assistant',
          message_type: 'System',
          content: `错误: ${response.error}`,
          created_at: new Date().toISOString(),
        };
        setLocalMessages(prev => [...prev, errorMessage]);
        setSending(false);
      } else {
        // Start polling to fetch results when background processing completes
        startPolling();
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      // Show error in chat
      const errorMessage: Message = {
        id: `temp-${Date.now() + 1}`,
        team_id: teamId,
        role: 'assistant',
        message_type: 'System',
        content: `错误: ${error instanceof Error ? error.message : 'Unknown error'}`,
        created_at: new Date().toISOString(),
      };
      setLocalMessages(prev => [...prev, errorMessage]);
      setSending(false);
    }
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
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-accent transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
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
          {sending && (
            <div className="flex gap-3 justify-start">
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-emerald-500 to-green-500 flex items-center justify-center">
                <Bot className="w-4 h-4 text-white" />
              </div>
              <div className="bg-muted rounded-2xl px-4 py-2.5">
                <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
              </div>
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div className="p-4 border-t border-border/50">
          <div className="flex gap-2">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="输入消息..."
              className="input-field flex-1 resize-none"
              rows={1}
              disabled={sending}
            />
            <button
              onClick={() => processing ? stopPolling() : handleSend()}
              disabled={!processing && !input.trim()}
              className={cn(
                'btn-primary px-4',
                (!processing && !input.trim()) ? 'opacity-50 cursor-not-allowed' : ''
              )}
            >
              {sending ? (
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
