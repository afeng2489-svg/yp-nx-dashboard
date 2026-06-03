import { useState, useRef, useCallback, useEffect } from 'react';
import { WS_BASE_URL, API_BASE_URL } from '@/api/constants';
import { unwrapEnvelope } from '@/api/response';

// 统一走 constants 的 API_BASE_URL：dev 走 proxy、生产直连 127.0.0.1:8080。
// 旧的 `import.meta.env.VITE_API_BASE_URL || ''` 打包后会变空串导致 execute-round 失败。
const API_BASE = API_BASE_URL;

/** 并行轮次执行 — 单个 bot 的状态 */
export interface ParallelBotState {
  role_id: string;
  execution_id: string;
  status: 'pending' | 'thinking' | 'done' | 'failed';
  elapsed_secs: number;
  role_name?: string;
  error_message?: string;
}

export interface UseParallelRoundReturn {
  bots: ParallelBotState[];
  isRunning: boolean;
  executeRound: (
    sessionId: string,
    roleIds: string[],
    getRoleName: (id: string) => string,
    onAllDone: (finalBots: ParallelBotState[]) => void,
  ) => Promise<void>;
  reset: () => void;
  /** 真正取消本轮：对仍在运行的执行按 id 通知后端取消（终止子进程），并清理本地状态 */
  cancel: () => void;
}

/** 并行轮次执行超时时间 (ms) */
const PARALLEL_TIMEOUT_MS = 180_000; // 3 minutes

/** 使用 parallel round hook — 管理多个并发执行 */
export function useParallelRound(): UseParallelRoundReturn {
  const [bots, setBots] = useState<ParallelBotState[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const wsRefs = useRef<Map<string, WebSocket>>(new Map());
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onAllDoneRef = useRef<((finalBots: ParallelBotState[]) => void) | null>(null);
  // 镜像 bots 以便在取消时读取仍在运行的执行 id（避免 stale closure）
  const botsRef = useRef<ParallelBotState[]>([]);
  botsRef.current = bots;

  // Detect when all bots reach terminal state (outside of state setter)
  useEffect(() => {
    if (!isRunning || bots.length === 0) return;
    const allDone = bots.every((b) => b.status === 'done' || b.status === 'failed');
    if (allDone) {
      setIsRunning(false);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      onAllDoneRef.current?.(bots);
      onAllDoneRef.current = null;
    }
  }, [bots, isRunning]);

  const executeRound = useCallback(
    async (
      sessionId: string,
      roleIds: string[],
      getRoleName: (id: string) => string,
      onAllDone: (finalBots: ParallelBotState[]) => void,
    ) => {
      if (roleIds.length === 0) return;

      // Close any lingering sockets
      wsRefs.current.forEach((ws) => ws.close());
      wsRefs.current.clear();

      setIsRunning(true);
      onAllDoneRef.current = onAllDone;

      const res = await fetch(`${API_BASE}/api/v1/group-sessions/${sessionId}/execute-round`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role_ids: roleIds }),
      });

      if (!res.ok) {
        setIsRunning(false);
        onAllDoneRef.current = null;
        throw new Error(`HTTP ${res.status}`);
      }

      const executions: { role_id: string; execution_id: string }[] = unwrapEnvelope(
        await res.json(),
      );

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

      // Set timeout for stuck executions
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => {
        setBots((prev) =>
          prev.map((b) =>
            b.status === 'pending' || b.status === 'thinking'
              ? { ...b, status: 'failed' as const, error_message: '执行超时' }
              : b,
          ),
        );
      }, PARALLEL_TIMEOUT_MS);

      for (const { execution_id } of executions) {
        const ws = new WebSocket(`${WS_BASE_URL}/ws/agent-executions/${execution_id}`);
        wsRefs.current.set(execution_id, ws);

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            setBots((prev) =>
              prev.map((b) => {
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
                    return {
                      ...b,
                      status: 'failed' as const,
                      error_message:
                        data.error ||
                        data.message ||
                        (data.type === 'cancelled' ? '执行被取消' : '执行失败'),
                    };
                  default:
                    return b;
                }
              }),
            );
          } catch {
            // ignore parse errors
          }
        };

        ws.onclose = () => wsRefs.current.delete(execution_id);
        ws.onerror = () => {
          setBots((prev) =>
            prev.map((b) =>
              b.execution_id === execution_id
                ? {
                    ...b,
                    status: 'failed' as const,
                    error_message: b.error_message || 'WebSocket 连接错误',
                  }
                : b,
            ),
          );
        };
      }
    },
    [],
  );

  const reset = useCallback(() => {
    wsRefs.current.forEach((ws) => ws.close());
    wsRefs.current.clear();
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    onAllDoneRef.current = null;
    setBots([]);
    setIsRunning(false);
  }, []);

  const cancel = useCallback(() => {
    // 对仍在运行的 bot 按 execution_id 通知后端真正取消（终止 Claude 子进程）
    botsRef.current
      .filter((b) => b.status === 'pending' || b.status === 'thinking')
      .forEach((b) => {
        void fetch(`${API_BASE}/api/v1/agent-executions/${b.execution_id}/cancel`, {
          method: 'POST',
        }).catch(() => {});
      });
    reset();
  }, [reset]);

  // 组件卸载时若仍有在跑的执行，确保后端被取消，避免子进程在后台继续污染项目
  useEffect(() => {
    return () => {
      botsRef.current
        .filter((b) => b.status === 'pending' || b.status === 'thinking')
        .forEach((b) => {
          void fetch(`${API_BASE}/api/v1/agent-executions/${b.execution_id}/cancel`, {
            method: 'POST',
          }).catch(() => {});
        });
      wsRefs.current.forEach((ws) => ws.close());
      wsRefs.current.clear();
    };
  }, []);

  return { bots, isRunning, executeRound, reset, cancel };
}
