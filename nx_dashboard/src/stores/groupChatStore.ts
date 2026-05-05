import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import { unwrapEnvelope, fetchWithTimeout } from '../api/response';

// Types matching the backend models
export type GroupStatus = 'pending' | 'active' | 'concluded';
export type SpeakingStrategy = 'free' | 'round_robin' | 'moderator' | 'debate';
export type ConsensusStrategy = 'majority' | 'unanimous' | 'score';

export interface GroupSession {
  id: string;
  team_id: string;
  name: string;
  topic: string;
  status: GroupStatus;
  speaking_strategy: SpeakingStrategy;
  consensus_strategy: ConsensusStrategy;
  moderator_role_id?: string;
  max_turns: number;
  current_turn: number;
  turn_policy: string;
  created_at: string;
  updated_at: string;
}

export interface GroupParticipant {
  role_id: string;
  role_name: string;
  joined_at: string;
  last_spoke_at?: string;
  message_count: number;
}

export interface GroupMessage {
  id: string;
  session_id: string;
  role_id: string;
  role_name: string;
  content: string;
  tool_calls: unknown[];
  reply_to?: string;
  turn_number: number;
  created_at: string;
}

export interface GroupConclusion {
  id: string;
  session_id: string;
  content: string;
  consensus_level: number;
  participant_scores: Record<string, number>;
  agreed_by: string[];
  created_at: string;
}

export interface GroupSessionDetail extends GroupSession {
  participants: GroupParticipant[];
  message_count: number;
  conclusion?: GroupConclusion;
}

export interface DiscussionTurnInfo {
  current_turn: number;
  max_turns: number;
  next_speaker_role_id?: string;
  next_speaker_role_name?: string;
  speaking_order: string[];
}

export interface CreateGroupSessionRequest {
  team_id: string;
  name: string;
  topic: string;
  speaking_strategy?: SpeakingStrategy;
  consensus_strategy?: ConsensusStrategy;
  moderator_role_id?: string;
  max_turns?: number;
  turn_policy?: string;
}

export interface UpdateGroupSessionRequest {
  name?: string;
  topic?: string;
  speaking_strategy?: SpeakingStrategy;
  consensus_strategy?: ConsensusStrategy;
  moderator_role_id?: string;
  max_turns?: number;
}

export interface StartDiscussionRequest {
  participant_role_ids: string[];
}

export interface SendMessageRequest {
  role_id: string;
  content: string;
  reply_to?: string;
}

export interface GetMessagesRequest {
  limit?: number;
  before?: string;
}

export interface ConcludeDiscussionRequest {
  force?: boolean;
}

// Custom error type
class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

interface GroupChatStore {
  sessions: GroupSession[];
  currentSession: GroupSessionDetail | null;
  messages: GroupMessage[];
  loading: boolean;
  error: string | null;

  // Session actions
  fetchSessions: (teamId?: string) => Promise<void>;
  fetchSession: (id: string) => Promise<GroupSessionDetail | null>;
  createSession: (request: CreateGroupSessionRequest) => Promise<GroupSession>;
  updateSession: (id: string, request: UpdateGroupSessionRequest) => Promise<GroupSession>;
  deleteSession: (id: string) => Promise<void>;

  // Discussion actions
  startDiscussion: (id: string, request: StartDiscussionRequest) => Promise<DiscussionTurnInfo>;
  sendMessage: (id: string, request: SendMessageRequest) => Promise<GroupMessage>;
  getNextSpeaker: (id: string) => Promise<{ role_id: string; role_name: string } | null>;
  advanceSpeaker: (id: string) => Promise<void>;
  executeRoleTurn: (id: string, roleId: string) => Promise<GroupMessage>;
  concludeDiscussion: (id: string, request?: ConcludeDiscussionRequest) => Promise<GroupConclusion>;

  // Message actions
  fetchMessages: (id: string, request?: GetMessagesRequest) => Promise<GroupMessage[]>;

  // UI state
  setCurrentSession: (session: GroupSessionDetail | null) => void;
  clearError: () => void;
}

export const useGroupChatStore = create<GroupChatStore>((set, get) => ({
  sessions: [],
  currentSession: null,
  messages: [],
  loading: false,
  error: null,

  fetchSessions: async (teamId?: string) => {
    set({ loading: true, error: null });
    try {
      const url = teamId
        ? `${API_BASE_URL}/api/v1/group-sessions?team_id=${encodeURIComponent(teamId)}`
        : `${API_BASE_URL}/api/v1/group-sessions`;
      const response = await fetchWithTimeout(url);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch sessions: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const data = unwrapEnvelope<GroupSession[]>(await response.json());
      set({ sessions: data, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch sessions: ${message}`,
      });
    }
  },

  fetchSession: async (id: string) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/group-sessions/${id}`);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch session: ${response.status}`, response.status);
      }

      const data: GroupSessionDetail = unwrapEnvelope(await response.json());
      set({ currentSession: data });
      return data;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to fetch session ${id}:`, message);
      return null;
    }
  },

  createSession: async (request: CreateGroupSessionRequest) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/group-sessions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create session: ${response.status}`, response.status);
      }

      const newSession: GroupSession = unwrapEnvelope(await response.json());
      set((state) => ({ sessions: [newSession, ...state.sessions] }));
      return newSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create session: ${message}` });
      throw error;
    }
  },

  updateSession: async (id: string, request: UpdateGroupSessionRequest) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/group-sessions/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to update session: ${response.status}`, response.status);
      }

      const updatedSession: GroupSession = unwrapEnvelope(await response.json());
      set((state) => ({
        sessions: state.sessions.map((s) => (s.id === id ? updatedSession : s)),
        currentSession:
          state.currentSession?.id === id
            ? { ...state.currentSession, ...updatedSession }
            : state.currentSession,
      }));
      return updatedSession;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to update session: ${message}` });
      throw error;
    }
  },

  deleteSession: async (id: string) => {
    // Optimistic update
    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== id),
      currentSession: state.currentSession?.id === id ? null : state.currentSession,
    }));

    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/group-sessions/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to delete session: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to delete session: ${message}` });
      throw error;
    }
  },

  startDiscussion: async (id: string, request: StartDiscussionRequest) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/group-sessions/${id}/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to start discussion: ${response.status}`, response.status);
      }

      return unwrapEnvelope(await response.json());
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to start discussion: ${message}` });
      throw error;
    }
  },

  sendMessage: async (id: string, request: SendMessageRequest) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/group-sessions/${id}/messages`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(request),
        },
      );

      if (!response.ok) {
        throw new ApiError(`Failed to send message: ${response.status}`, response.status);
      }

      const message: GroupMessage = unwrapEnvelope(await response.json());
      set((state) => ({ messages: [...state.messages, message] }));
      return message;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to send message: ${message}` });
      throw error;
    }
  },

  getNextSpeaker: async (id: string) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/group-sessions/${id}/next-speaker`,
      );

      if (!response.ok) {
        throw new ApiError(`Failed to get next speaker: ${response.status}`, response.status);
      }

      const data = unwrapEnvelope<{ role_id: string; role_name: string } | null>(
        await response.json(),
      );
      return data; // { role_id, role_name } or null
    } catch (error) {
      console.error(`Failed to get next speaker for ${id}:`, error);
      return null;
    }
  },

  advanceSpeaker: async (id: string) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/group-sessions/${id}/advance`,
        { method: 'POST' },
      );

      if (!response.ok) {
        throw new ApiError(`Failed to advance speaker: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to advance speaker: ${message}` });
      throw error;
    }
  },

  executeRoleTurn: async (id: string, roleId: string) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/group-sessions/${id}/execute-turn/${encodeURIComponent(roleId)}`,
        { method: 'POST' },
      );

      if (!response.ok) {
        throw new ApiError(`Failed to execute role turn: ${response.status}`, response.status);
      }

      const message: GroupMessage = unwrapEnvelope(await response.json());
      set((state) => ({ messages: [...state.messages, message] }));
      return message;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to execute role turn: ${message}` });
      throw error;
    }
  },

  concludeDiscussion: async (id: string, request: ConcludeDiscussionRequest = {}) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/group-sessions/${id}/conclude`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(request),
        },
      );

      if (!response.ok) {
        throw new ApiError(`Failed to conclude discussion: ${response.status}`, response.status);
      }

      const conclusion: GroupConclusion = unwrapEnvelope(await response.json());
      // Refresh the session to get updated status
      get().fetchSession(id);
      return conclusion;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to conclude discussion: ${message}` });
      throw error;
    }
  },

  fetchMessages: async (id: string, request: GetMessagesRequest = {}) => {
    try {
      const params = new URLSearchParams();
      if (request.limit) params.set('limit', String(request.limit));
      if (request.before) params.set('before', request.before);

      const url = `${API_BASE_URL}/api/v1/group-sessions/${id}/messages${
        params.toString() ? `?${params.toString()}` : ''
      }`;
      const response = await fetchWithTimeout(url);

      if (!response.ok) {
        throw new ApiError(`Failed to fetch messages: ${response.status}`, response.status);
      }

      const messages: GroupMessage[] = unwrapEnvelope(await response.json());
      set({ messages });
      return messages;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to fetch messages for ${id}:`, message);
      return [];
    }
  },

  setCurrentSession: (session) => set({ currentSession: session }),

  clearError: () => set({ error: null }),
}));
