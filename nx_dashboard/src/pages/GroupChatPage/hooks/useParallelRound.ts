import { useState, useRef, useCallback } from 'react';
import { WS_BASE_URL } from '@/api/constants';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

/** 并行轮次执行 — 单个 bot 的状态 */
export interface ParallelBotState {
  role_id: string;
  execution_id: string;
  status: 'pending' | 'thinking' | 'done' | 'failed';
  elapsed_secs: number;
  role_name?: string;
}

export interface UseParallelRoundReturn {
  bots: ParallelBotState[];
  isRunning: boolean;
  executeRound: (
    sessionId: string,
    roleIds: string[],
    getRoleName: (id: string) => string,
    onAllDone: () => void,
  ) => Promise<void>;
  reset: () => void;
}

/** 使用 parallel round hook — 管理多个并发执行 */
export function useParallelRound(): UseParallelRoundReturn {
  const [bots, setBots] = useState<ParallelBotState[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const wsRefs = useRef<Map<string, WebSocket>>(new Map());

  const executeRound = useCallback(
    async (
      sessionId: string,
      roleIds: string[],
      getRoleName: (id: string) => string,
      onAllDone: () => void,
    ) => {
      if (roleIds.length === 0) return;

      // Close any lingering sockets
      wsRefs.current.forEach((ws) => ws.close());
      wsRefs.current.clear();

      setIsRunning(true);

      const res = await fetch(`${API_BASE}/api/v1/group-sessions/${sessionId}/execute-round`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role_ids: roleIds }),
      });

      if (!res.ok) {
        setIsRunning(false);
        throw new Error(`HTTP ${res.status}`);
      }

      const executions: { role_id: string; execution_id: string }[] = await res.json();

      // Initialise state for all bots
      setBots(
        executions.map(({ role_id, execution_id }) => ({
          role_id,
          execution_id,
          status: 'pending',
          elapsed_secs: 0,
          role_name: getRoleName(role_id),
        })),
      );

      const checkAllDone = (prev: ParallelBotState[]) => {
        if (prev.every((b) => b.status === 'done' || b.status === 'failed')) {
          setIsRunning(false);
          onAllDone();
        }
      };

      for (const { execution_id } of executions) {
        const ws = new WebSocket(`${WS_BASE_URL}/ws/agent-executions/${execution_id}`);
        wsRefs.current.set(execution_id, ws);

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            setBots((prev) => {
              const next = prev.map((b) => {
                if (b.execution_id !== execution_id) return b;
                switch (data.type) {
                  case 'started':
                    return { ...b, status: 'pending' as const };
                  case 'thinking':
                    return {
                      ...b,
                      status: 'thinking' as const,
                      elapsed_secs: data.elapsed_secs ?? b.elapsed_secs,
                    };
                  case 'completed':
                    return { ...b, status: 'done' as const };
                  case 'failed':
                  case 'cancelled':
                    return { ...b, status: 'failed' as const };
                  default:
                    return b;
                }
              });
              checkAllDone(next);
              return next;
            });
          } catch {
            // ignore parse errors
          }
        };

        ws.onclose = () => wsRefs.current.delete(execution_id);
        ws.onerror = () => {
          setBots((prev) => {
            const next = prev.map((b) =>
              b.execution_id === execution_id ? { ...b, status: 'failed' as const } : b,
            );
            checkAllDone(next);
            return next;
          });
        };
      }
    },
    [],
  );

  const reset = useCallback(() => {
    wsRefs.current.forEach((ws) => ws.close());
    wsRefs.current.clear();
    setBots([]);
    setIsRunning(false);
  }, []);

  return { bots, isRunning, executeRound, reset };
}
