import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

export type SessionStatus = 'pending' | 'running' | 'active' | 'idle' | 'paused' | 'terminated';

export interface Session {
  id: string;
  workflow_id?: string;
  status: SessionStatus;
  resume_key?: string;
  created_at: string;
  updated_at: string;
}

// 自定义错误类型
class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// 带 timeout 的 fetch
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 5000
): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    clearTimeout(timeoutId);
    return response;
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === 'AbortError') {
      throw new ApiError('Request timeout', 408);
    }
    throw error;
  }
}

interface SessionStore {
  sessions: Session[];
  currentSession: Session | null;
  loading: boolean;
  error: string | null;

  fetchSessions: () => Promise<void>;
  getSession: (id: string) => Promise<Session | null>;
  createSession: (workflowId: string) => Promise<Session>;
  resumeSession: (resumeKey: string) => Promise<Session | null>;
  pauseSession: (id: string) => Promise<Session | null>;
  activateSession: (id: string) => Promise<Session | null>;
  syncSession: (id: string) => Promise<Session | null>;
  terminateSession: (id: string) => Promise<void>;
  setCurrentSession: (session: Session | null) => void;
  clearError: () => void;
}

export const useSessionStore = create<SessionStore>((set) => ({
  sessions: [],
  currentSession: null,
  loading: false,
  error: null,

  fetchSessions: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/sessions`);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch sessions: ${response.status} ${response.statusText}`,
          response.status
        );
      }

      const data = await response.json();
      set({ sessions: data, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch sessions: ${message}`,
      });
    }
  },

  getSession: async (id) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/sessions/${id}`);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(
          `Failed to fetch session: ${response.status}`,
          response.status
        );
      }

      return await response.json();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get session ${id}:`, message);
      return null;
    }
  },

  createSession: async (workflowId) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/sessions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ workflow_id: workflowId }),
      });

      if (!response.ok) {
        throw new ApiError(
          `Failed to create session: ${response.status}`,
          response.status
        );
      }

      const newSession = await response.json();
      set((state) => ({ sessions: [...state.sessions, newSession] }));
      return newSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create session: ${message}` });
      throw error;
    }
  },

  resumeSession: async (resumeKey) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/sessions/resume/${resumeKey}`,
        { method: 'POST' }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to resume session: ${response.status}`,
          response.status
        );
      }

      const resumedSession = await response.json();
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === resumedSession.id ? resumedSession : s
        ),
        currentSession:
          state.currentSession?.id === resumedSession.id
            ? resumedSession
            : state.currentSession,
      }));
      return resumedSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to resume session:`, message);
      return null;
    }
  },

  pauseSession: async (id) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/sessions/${id}/pause`,
        { method: 'POST' }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to pause session: ${response.status}`,
          response.status
        );
      }

      const pausedSession = await response.json();
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === pausedSession.id ? pausedSession : s
        ),
        currentSession:
          state.currentSession?.id === pausedSession.id
            ? pausedSession
            : state.currentSession,
      }));
      return pausedSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to pause session:`, message);
      return null;
    }
  },

  activateSession: async (id) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/sessions/${id}/activate`,
        { method: 'POST' }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to activate session: ${response.status}`,
          response.status
        );
      }

      const activatedSession = await response.json();
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === activatedSession.id ? activatedSession : s
        ),
        currentSession:
          state.currentSession?.id === activatedSession.id
            ? activatedSession
            : state.currentSession,
      }));
      return activatedSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to activate session:`, message);
      return null;
    }
  },

  syncSession: async (id) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/sessions/${id}/sync`,
        { method: 'POST' }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to sync session: ${response.status}`,
          response.status
        );
      }

      const syncedSession = await response.json();
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === syncedSession.id ? syncedSession : s
        ),
        currentSession:
          state.currentSession?.id === syncedSession.id
            ? syncedSession
            : state.currentSession,
      }));
      return syncedSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to sync session:`, message);
      return null;
    }
  },

  terminateSession: async (id) => {
    // Optimistic update
    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== id),
      currentSession: state.currentSession?.id === id ? null : state.currentSession,
    }));

    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/sessions/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(
          `Failed to terminate session: ${response.status}`,
          response.status
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  setCurrentSession: (session) => set({ currentSession: session }),

  clearError: () => set({ error: null }),
}));