import { create } from 'zustand';
import { API_BASE_URL, WS_BASE_URL } from '../api/constants';
import type {
  InteractiveMessage,
  A2UISession,
  U2AMessage,
  A2UIWsEvent,
} from '@/components/a2ui/types';

// API_BASE_URL and WS_BASE_URL are imported from constants

/**
 * Custom error for API errors
 */
class A2UIError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: string,
  ) {
    super(message);
    this.name = 'A2UIError';
  }
}

/**
 * A2UI Store interface
 */
interface A2UIStore {
  // State
  sessions: Map<string, A2UISession>;
  messages: Map<string, InteractiveMessage[]>;
  pendingMessages: Map<string, InteractiveMessage[]>;
  currentSessionId: string | null;
  loading: boolean;
  error: string | null;
  wsConnections: Map<string, WebSocket>;

  // Actions
  getOrCreateSession: (executionId: string) => Promise<A2UISession>;
  fetchMessages: (sessionId: string) => Promise<InteractiveMessage[]>;
  respondToMessage: (
    sessionId: string,
    messageId: string,
    response: U2AMessage,
  ) => Promise<InteractiveMessage>;
  connectWebSocket: (executionId: string) => void;
  disconnectWebSocket: (executionId: string) => void;
  clearError: () => void;
  setCurrentSession: (sessionId: string | null) => void;
}

export const useA2UIStore = create<A2UIStore>((set, get) => ({
  sessions: new Map(),
  messages: new Map(),
  pendingMessages: new Map(),
  currentSessionId: null,
  loading: false,
  error: null,
  wsConnections: new Map(),

  getOrCreateSession: async (executionId: string) => {
    // Check if we already have a session for this execution
    const { sessions } = get();
    for (const [sessionId, session] of sessions) {
      if (session.execution_id === executionId && session.state !== 'ended') {
        return session;
      }
    }

    set({ loading: true, error: null });
    try {
      // Create new session via WebSocket or get existing
      const response = await fetch(
        `${API_BASE_URL}/api/v1/a2ui/sessions?execution_id=${executionId}`,
        {
          method: 'GET',
          headers: { 'Content-Type': 'application/json' },
        },
      );

      if (!response.ok) {
        throw new A2UIError(`Failed to get session: ${response.status}`, response.status);
      }

      const sessions: A2UISession[] = await response.json();

      // Find existing or use first
      const existingSession = sessions.find(
        (s) => s.execution_id === executionId && s.state !== 'ended',
      );

      if (existingSession) {
        set((state) => ({
          sessions: new Map(state.sessions).set(existingSession.id, existingSession),
          currentSessionId: existingSession.id,
          loading: false,
        }));
        return existingSession;
      }

      // No session exists, return a placeholder for WebSocket connection to handle
      set({ loading: false });
      return {
        id: '',
        execution_id: executionId,
        state: 'waiting' as const,
      };
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      set({ loading: false, error: `Failed to get session: ${message}` });
      throw err;
    }
  },

  fetchMessages: async (sessionId: string) => {
    set({ loading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/a2ui/sessions/${sessionId}/messages`, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' },
      });

      if (!response.ok) {
        throw new A2UIError(`Failed to fetch messages: ${response.status}`, response.status);
      }

      const data = await response.json();
      const messages = data.messages as InteractiveMessage[];
      const pending = messages.filter((m) => m.pending);

      set((state) => ({
        messages: new Map(state.messages).set(sessionId, messages),
        pendingMessages: new Map(state.pendingMessages).set(sessionId, pending),
        loading: false,
      }));

      return messages;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      set({ loading: false, error: `Failed to fetch messages: ${message}` });
      throw err;
    }
  },

  respondToMessage: async (sessionId: string, messageId: string, response: U2AMessage) => {
    set({ error: null });
    try {
      const fetchResponse = await fetch(
        `${API_BASE_URL}/api/v1/a2ui/sessions/${sessionId}/respond`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            message_id: messageId,
            response,
          }),
        },
      );

      if (!fetchResponse.ok) {
        throw new A2UIError(
          `Failed to send response: ${fetchResponse.status}`,
          fetchResponse.status,
        );
      }

      const updatedMessage: InteractiveMessage = await fetchResponse.json();

      // Update local state
      set((state) => {
        const messages = state.messages.get(sessionId) || [];
        const updatedMessages = messages.map((m) => (m.id === messageId ? updatedMessage : m));

        const pendingMessages = (state.pendingMessages.get(sessionId) || []).filter(
          (m) => m.id !== messageId,
        );

        return {
          messages: new Map(state.messages).set(sessionId, updatedMessages),
          pendingMessages: new Map(state.pendingMessages).set(sessionId, pendingMessages),
        };
      });

      return updatedMessage;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      set({ error: `Failed to respond: ${message}` });
      throw err;
    }
  },

  connectWebSocket: (executionId: string) => {
    const { wsConnections } = get();
    if (wsConnections.has(executionId)) return;

    const ws = new WebSocket(`${WS_BASE_URL}/ws/a2ui/${executionId}`);

    ws.onopen = () => {
      console.log(`[A2UI WS] Connected to execution ${executionId}`);
    };

    ws.onmessage = (event) => {
      try {
        const wsEvent: A2UIWsEvent = JSON.parse(event.data);
        handleWsEvent(wsEvent, executionId);
      } catch (e) {
        console.error('[A2UI WS] Failed to parse message:', e);
      }
    };

    ws.onclose = () => {
      console.log(`[A2UI WS] Disconnected from execution ${executionId}`);
      wsConnections.delete(executionId);
    };

    ws.onerror = (error) => {
      console.error(`[A2UI WS] Error for execution ${executionId}:`, error);
    };

    wsConnections.set(executionId, ws);
  },

  disconnectWebSocket: (executionId: string) => {
    const { wsConnections } = get();
    const ws = wsConnections.get(executionId);
    if (ws) {
      ws.close();
      wsConnections.delete(executionId);
    }
  },

  clearError: () => set({ error: null }),

  setCurrentSession: (sessionId: string | null) => set({ currentSessionId: sessionId }),
}));

// Helper function to handle WebSocket events
function handleWsEvent(event: A2UIWsEvent, executionId: string) {
  const store = useA2UIStore.getState();

  switch (event.type) {
    case 'message':
    case 'pending':
      if (event.data) {
        const sessionId = event.data.session_id;
        const messages = store.messages.get(sessionId) || [];
        const existingIndex = messages.findIndex((m) => m.id === event.data!.id);

        let updatedMessages: InteractiveMessage[];
        if (existingIndex >= 0) {
          updatedMessages = [...messages];
          updatedMessages[existingIndex] = event.data;
        } else {
          updatedMessages = [...messages, event.data];
        }

        const pendingMessages = updatedMessages.filter((m) => m.pending);

        useA2UIStore.setState((state) => ({
          messages: new Map(state.messages).set(sessionId, updatedMessages),
          pendingMessages: new Map(state.pendingMessages).set(sessionId, pendingMessages),
        }));
      }
      break;

    case 'response':
      if (event.message_id && event.response) {
        // Handle user response confirmation
        console.log('[A2UI WS] Response sent:', event.message_id);
      }
      break;

    case 'ended':
      if (event.execution_id) {
        const { sessions } = useA2UIStore.getState();
        for (const [sessionId, session] of sessions) {
          if (session.execution_id === event.execution_id) {
            const updatedSession = { ...session, state: 'ended' as const };
            useA2UIStore.setState((state) => ({
              sessions: new Map(state.sessions).set(sessionId, updatedSession),
            }));
            break;
          }
        }
      }
      break;

    case 'error':
      console.error('[A2UI WS] Error:', event.error);
      useA2UIStore.setState({
        error: event.error?.message || 'Unknown WebSocket error',
      });
      break;
  }
}

// Selector hooks for optimized re-renders
export const useA2UISession = (executionId: string) =>
  useA2UIStore((state) => {
    for (const session of state.sessions.values()) {
      if (session.execution_id === executionId && session.state !== 'ended') {
        return session;
      }
    }
    return null;
  });

export const useA2UIPendingMessages = (sessionId: string | null) =>
  useA2UIStore((state) => (sessionId ? state.pendingMessages.get(sessionId) || [] : []));

export const useA2UIMessages = (sessionId: string | null) =>
  useA2UIStore((state) => (sessionId ? state.messages.get(sessionId) || [] : []));
