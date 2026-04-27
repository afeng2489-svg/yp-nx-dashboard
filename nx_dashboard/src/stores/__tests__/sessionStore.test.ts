import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSessionStore, Session, SessionStatus } from '../sessionStore';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('useSessionStore', () => {
  const mockSession: Session = {
    id: 'session-1',
    workflow_id: 'workflow-1',
    status: 'pending',
    resume_key: 'resume-key-1',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    useSessionStore.setState({
      sessions: [],
      currentSession: null,
      loading: false,
      error: null,
    });
  });

  describe('initial state', () => {
    it('should have empty sessions array', () => {
      const { result } = renderHook(() => useSessionStore());
      expect(result.current.sessions).toEqual([]);
    });

    it('should have null currentSession', () => {
      const { result } = renderHook(() => useSessionStore());
      expect(result.current.currentSession).toBeNull();
    });

    it('should have loading false', () => {
      const { result } = renderHook(() => useSessionStore());
      expect(result.current.loading).toBe(false);
    });

    it('should have null error', () => {
      const { result } = renderHook(() => useSessionStore());
      expect(result.current.error).toBeNull();
    });
  });

  describe('fetchSessions', () => {
    it('should fetch sessions successfully', async () => {
      const sessions = [mockSession];
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve(sessions),
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        await result.current.fetchSessions();
      });

      expect(result.current.sessions).toEqual(sessions);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it('should handle fetch error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        await result.current.fetchSessions();
      });

      expect(result.current.sessions).toEqual([]);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toContain('Failed to fetch sessions');
    });
  });

  describe('createSession', () => {
    it('should create a new session', async () => {
      const newSession = { ...mockSession, id: 'session-2' };
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: () => Promise.resolve(newSession),
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        const session = await result.current.createSession('workflow-1');
        expect(session).toEqual(newSession);
      });

      expect(result.current.sessions).toContainEqual(newSession);
    });

    it('should handle create error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        try {
          await result.current.createSession('workflow-1');
        } catch {
          // Expected to throw
        }
      });

      expect(result.current.error).toContain('Failed to create session');
    });
  });

  describe('getSession', () => {
    it('should get a session by id', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve(mockSession),
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        const session = await result.current.getSession('session-1');
        expect(session).toEqual(mockSession);
      });
    });

    it('should return null for 404', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        const session = await result.current.getSession('non-existent');
        expect(session).toBeNull();
      });
    });
  });

  describe('pauseSession', () => {
    it('should pause a session', async () => {
      const pausedSession = { ...mockSession, status: 'paused' as SessionStatus };
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve(pausedSession),
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        const session = await result.current.pauseSession('session-1');
        expect(session?.status).toBe('paused');
      });
    });
  });

  describe('activateSession', () => {
    it('should activate a session', async () => {
      const activeSession = { ...mockSession, status: 'active' as SessionStatus };
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve(activeSession),
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        const session = await result.current.activateSession('session-1');
        expect(session?.status).toBe('active');
      });
    });
  });

  describe('terminateSession', () => {
    it('should terminate a session', async () => {
      // Set initial session
      useSessionStore.setState({ sessions: [mockSession] });

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 204,
      });

      const { result } = renderHook(() => useSessionStore());

      await act(async () => {
        await result.current.terminateSession('session-1');
      });

      expect(result.current.sessions).not.toContainEqual(
        expect.objectContaining({ id: 'session-1' }),
      );
    });
  });

  describe('setCurrentSession', () => {
    it('should set current session', () => {
      const { result } = renderHook(() => useSessionStore());

      act(() => {
        result.current.setCurrentSession(mockSession);
      });

      expect(result.current.currentSession).toEqual(mockSession);
    });

    it('should clear current session with null', () => {
      useSessionStore.setState({ currentSession: mockSession });

      const { result } = renderHook(() => useSessionStore());

      act(() => {
        result.current.setCurrentSession(null);
      });

      expect(result.current.currentSession).toBeNull();
    });
  });

  describe('clearError', () => {
    it('should clear error', () => {
      useSessionStore.setState({ error: 'Some error' });

      const { result } = renderHook(() => useSessionStore());

      act(() => {
        result.current.clearError();
      });

      expect(result.current.error).toBeNull();
    });
  });
});
