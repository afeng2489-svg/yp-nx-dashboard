import { useState, useEffect, useCallback } from 'react';
import { useA2UIStore } from '@/stores/a2uiStore';
import type { A2UIPanelProps, InteractiveMessage, U2AMessage } from './types';
import {
  MessageSquare,
  AlertCircle,
  CheckCircle,
  XCircle,
  Loader2,
  Send,
  ChevronRight,
  Clock,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// Message type icons
const MESSAGE_ICONS = {
  ask: <MessageSquare className="w-4 h-4 text-blue-500" />,
  inform: <MessageSquare className="w-4 h-4 text-gray-500" />,
  confirm: <AlertCircle className="w-4 h-4 text-yellow-500" />,
  select: <ChevronRight className="w-4 h-4 text-purple-500" />,
} as const;

// Inform level colors
const INFORM_COLORS = {
  info: 'bg-blue-50 border-blue-200 text-blue-900',
  warning: 'bg-yellow-50 border-yellow-200 text-yellow-900',
  error: 'bg-red-50 border-red-200 text-red-900',
  success: 'bg-green-50 border-green-200 text-green-900',
} as const;

/**
 * Ask message component
 */
function AskMessage({
  message,
  onRespond,
  disabled,
}: {
  message: InteractiveMessage;
  onRespond: (response: U2AMessage) => void;
  disabled: boolean;
}) {
  const [inputValue, setInputValue] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (inputValue.trim()) {
      onRespond({ type: 'response', value: inputValue.trim() });
      setInputValue('');
    }
  };

  return (
    <div className="space-y-3">
      {message.content.context && (
        <div className="text-sm text-muted-foreground bg-muted/50 p-2 rounded">
          {message.content.context}
        </div>
      )}
      <div className="text-sm font-medium">{message.content.question}</div>
      <form onSubmit={handleSubmit} className="flex gap-2">
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          disabled={disabled}
          placeholder="Type your answer..."
          className="flex-1 px-3 py-2 text-sm border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 disabled:opacity-50"
        />
        <button
          type="submit"
          disabled={disabled || !inputValue.trim()}
          className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
        >
          <Send className="w-4 h-4" />
          Send
        </button>
      </form>
    </div>
  );
}

/**
 * Confirm message component
 */
function ConfirmMessage({
  message,
  onRespond,
  disabled,
}: {
  message: InteractiveMessage;
  onRespond: (response: U2AMessage) => void;
  disabled: boolean;
}) {
  return (
    <div className="space-y-3">
      <div className="text-sm font-medium">{message.content.prompt}</div>
      {message.content.details && (
        <div className="text-sm text-muted-foreground bg-muted/50 p-2 rounded">
          {message.content.details}
        </div>
      )}
      <div className="flex gap-2">
        <button
          onClick={() => onRespond({ type: 'confirm' })}
          disabled={disabled}
          className="flex-1 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 flex items-center justify-center gap-2"
        >
          <CheckCircle className="w-4 h-4" />
          Confirm
        </button>
        <button
          onClick={() => onRespond({ type: 'cancel' })}
          disabled={disabled}
          className="flex-1 px-4 py-2 text-sm border border-input bg-background rounded-md hover:bg-accent disabled:opacity-50 flex items-center justify-center gap-2"
        >
          <XCircle className="w-4 h-4" />
          Cancel
        </button>
      </div>
    </div>
  );
}

/**
 * Select message component
 */
function SelectMessage({
  message,
  onRespond,
  disabled,
}: {
  message: InteractiveMessage;
  onRespond: (response: U2AMessage) => void;
  disabled: boolean;
}) {
  return (
    <div className="space-y-3">
      <div className="text-sm font-medium">{message.content.prompt}</div>
      <div className="space-y-2">
        {message.content.options?.map((option, index) => (
          <button
            key={index}
            onClick={() => onRespond({ type: 'select', value: index })}
            disabled={disabled}
            className="w-full px-4 py-3 text-sm text-left border rounded-md bg-background hover:bg-accent disabled:opacity-50 flex items-center gap-3"
          >
            <span className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium">
              {index + 1}
            </span>
            <span>{option}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * Inform message component
 */
function InformMessage({ message }: { message: InteractiveMessage }) {
  const level = message.content.level || 'info';
  const colorClass = INFORM_COLORS[level];

  return (
    <div className={cn('p-3 rounded-md border text-sm', colorClass)}>
      <div className="flex items-start gap-2">
        {level === 'error' && <XCircle className="w-4 h-4 mt-0.5" />}
        {level === 'warning' && <AlertCircle className="w-4 h-4 mt-0.5" />}
        {level === 'success' && <CheckCircle className="w-4 h-4 mt-0.5" />}
        {level === 'info' && <MessageSquare className="w-4 h-4 mt-0.5" />}
        <span>{message.content.message}</span>
      </div>
    </div>
  );
}

/**
 * Message bubble component
 */
function MessageBubble({
  message,
  onRespond,
  disabled,
}: {
  message: InteractiveMessage;
  onRespond: (response: U2AMessage) => void;
  disabled: boolean;
}) {
  const messageType = message.content.type;

  return (
    <div
      className={cn(
        'p-4 rounded-lg border',
        message.pending ? 'bg-yellow-50 border-yellow-200' : 'bg-card border-border',
      )}
    >
      <div className="flex items-center gap-2 mb-3">
        {MESSAGE_ICONS[messageType]}
        <span className="text-xs font-medium text-muted-foreground">{message.source}</span>
        <span className="text-xs text-muted-foreground">
          {new Date(message.timestamp).toLocaleTimeString()}
        </span>
        {message.pending && (
          <span className="ml-auto flex items-center gap-1 text-xs text-yellow-600">
            <Clock className="w-3 h-3" />
            Waiting for response
          </span>
        )}
      </div>

      {messageType === 'ask' && (
        <AskMessage message={message} onRespond={onRespond} disabled={disabled} />
      )}
      {messageType === 'confirm' && (
        <ConfirmMessage message={message} onRespond={onRespond} disabled={disabled} />
      )}
      {messageType === 'select' && (
        <SelectMessage message={message} onRespond={onRespond} disabled={disabled} />
      )}
      {messageType === 'inform' && <InformMessage message={message} />}
    </div>
  );
}

/**
 * A2UI Panel component for interactive agent-to-user communication
 */
export function A2UIPanel({ executionId, className }: A2UIPanelProps) {
  const {
    connectWebSocket,
    disconnectWebSocket,
    getOrCreateSession,
    fetchMessages,
    respondToMessage,
    error,
    clearError,
  } = useA2UIStore();

  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<InteractiveMessage[]>([]);
  const [pendingCount, setPendingCount] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Initialize session and WebSocket
  useEffect(() => {
    if (!executionId) return;

    const initSession = async () => {
      setIsLoading(true);
      try {
        const session = await getOrCreateSession(executionId);
        if (session.id) {
          setSessionId(session.id);
          const msgs = await fetchMessages(session.id);
          setMessages(msgs);
          setPendingCount(msgs.filter((m) => m.pending).length);
        }
      } catch (err) {
        console.error('Failed to initialize A2UI session:', err);
      } finally {
        setIsLoading(false);
      }
    };

    initSession();
    connectWebSocket(executionId);

    return () => {
      disconnectWebSocket(executionId);
    };
  }, [executionId, getOrCreateSession, fetchMessages, connectWebSocket, disconnectWebSocket]);

  // Handle response submission
  const handleRespond = useCallback(
    async (messageId: string, response: U2AMessage) => {
      if (!sessionId) return;

      setIsSubmitting(true);
      try {
        const updatedMessage = await respondToMessage(sessionId, messageId, response);
        setMessages((prev) => prev.map((m) => (m.id === messageId ? updatedMessage : m)));
        setPendingCount((prev) => Math.max(0, prev - 1));
      } catch (err) {
        console.error('Failed to send response:', err);
      } finally {
        setIsSubmitting(false);
      }
    },
    [sessionId, respondToMessage],
  );

  // Update messages when store changes
  useEffect(() => {
    if (!sessionId) return;

    const unsubscribe = useA2UIStore.subscribe((state) => {
      const sessionMessages = state.messages.get(sessionId) || [];
      if (sessionMessages.length !== messages.length) {
        setMessages(sessionMessages);
        setPendingCount(sessionMessages.filter((m) => m.pending).length);
      }
    });

    return unsubscribe;
  }, [sessionId, messages.length]);

  if (isLoading) {
    return (
      <div className={cn('flex items-center justify-center p-8', className)}>
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className={cn('p-4', className)}>
        <div className="flex items-center gap-2 text-red-500 text-sm">
          <AlertCircle className="w-4 h-4" />
          <span>{error}</span>
          <button onClick={clearError} className="ml-auto text-xs underline">
            Dismiss
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col', className)}>
      {/* Header */}
      <div className="px-4 py-3 border-b bg-accent/50">
        <div className="flex items-center justify-between">
          <h3 className="font-semibold text-sm">Interactive Messages</h3>
          {pendingCount > 0 && (
            <span className="px-2 py-1 text-xs bg-yellow-100 text-yellow-800 rounded-full">
              {pendingCount} pending
            </span>
          )}
        </div>
      </div>

      {/* Messages list */}
      <div className="flex-1 overflow-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground text-sm">
            No messages yet. The agent will send messages here when interaction is needed.
          </div>
        ) : (
          messages.map((message) => (
            <MessageBubble
              key={message.id}
              message={message}
              onRespond={(response) => handleRespond(message.id, response)}
              disabled={isSubmitting}
            />
          ))
        )}
      </div>
    </div>
  );
}
