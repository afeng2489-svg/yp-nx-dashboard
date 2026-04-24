import { useState, useRef, useCallback, useEffect } from 'react';
import { WS_BASE_URL } from '@/api/constants';
import { useTeamStore } from '@/stores/teamStore';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

/** Agent execution event from WebSocket */
interface AgentExecutionEvent {
  type: 'started' | 'thinking' | 'output' | 'completed' | 'failed' | 'cancelled' | 'confirmation_required';
  execution_id: string;
  agent_role?: string;
  task_summary?: string;
  elapsed_secs?: number;
  partial_output?: string;
  result?: string;
  duration_ms?: number;
  error?: string;
  question?: string;
  options?: string[];
  needs_input?: boolean;
  role_id?: string;
  session_id?: string;
}

export type AgentExecutionStatus =
  | 'idle'
  | 'started'
  | 'thinking'
  | 'confirmation'  // waiting for user confirmation
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface UseAgentExecutionReturn {
  executionId: string | null;
  status: AgentExecutionStatus;
  elapsedSecs: number;
  partialOutput: string;
  result: string | null;
  error: string | null;
  durationMs: number | null;
  confirmationQuestion: string | null;
  confirmationOptions: string[];
  activeRoleId: string | null;
  activeSessionId: string | null;
  execute: (teamId: string, task: string, autoConfirm?: boolean) => Promise<string | null>;
  executeRoleTurn: (sessionId: string, roleId: string) => Promise<string | null>;
  sendConfirmation: (response: string) => void;
  cancel: () => void;
  reset: () => void;
}

/**
 * Hook for async agent execution with WebSocket progress tracking.
 *
 * Flow:
 * 1. execute() sends POST, gets execution_id immediately
 * 2. Opens WS to /ws/agent-executions/{execution_id}
 * 3. Receives Thinking/Output/Completed/Failed events
 * 4. cancel() sends Cancel message via WS
 */
export function useAgentExecution(): UseAgentExecutionReturn {
  const [executionId, setExecutionId] = useState<string | null>(null);
  const [status, setStatus] = useState<AgentExecutionStatus>('idle');
  const [elapsedSecs, setElapsedSecs] = useState(0);
  const [partialOutput, setPartialOutput] = useState('');
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [durationMs, setDurationMs] = useState<number | null>(null);
  const [confirmationQuestion, setConfirmationQuestion] = useState<string | null>(null);
  const [confirmationOptions, setConfirmationOptions] = useState<string[]>([]);
  const [activeRoleId, setActiveRoleId] = useState<string | null>(null);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const timerRef = useRef<number | null>(null);
  const startTimeRef = useRef<number>(0);
  // Track whether an execution is in-flight so we don't close the WS on unmount
  const isRunningRef = useRef(false);
  // For reconnect: remember the last execId and monitorCtx
  const reconnectTimerRef = useRef<number | null>(null);
  const reconnectCountRef = useRef(0);
  const MAX_RECONNECT = 5;
  const lastExecIdRef = useRef<string | null>(null);
  const lastMonitorCtxRef = useRef<{ teamId: string; teamName: string; task: string } | null>(null);

  // Local elapsed timer (updates every second, independent of WS heartbeat)
  const startLocalTimer = useCallback(() => {
    startTimeRef.current = Date.now();
    timerRef.current = window.setInterval(() => {
      setElapsedSecs(Math.floor((Date.now() - startTimeRef.current) / 1000));
    }, 1000);
  }, []);

  const stopLocalTimer = useCallback(() => {
    if (timerRef.current !== null) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  // Cleanup on unmount — keep WS alive if task is still running so the
  // store update (setActiveTeamTask) fires even after the component closes
  useEffect(() => {
    return () => {
      stopLocalTimer();
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      if (!isRunningRef.current) {
        wsRef.current?.close();
      }
    };
  }, [stopLocalTimer]);

  /** Connect to WS and listen for events, with auto-reconnect on drop */
  const connectWs = useCallback((execId: string, monitorCtx: { teamId: string; teamName: string; task: string } | null = null) => {
    // Store for reconnect
    lastExecIdRef.current = execId;
    lastMonitorCtxRef.current = monitorCtx;

    const wsUrl = `${WS_BASE_URL}/ws/agent-executions/${execId}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;
    let accOutput = ''; // 累积 partial output（仅用于监控卡）
    let hasReceivedMessage = false; // distinguish initial vs runtime errors

    const scheduleReconnect = () => {
      if (!isRunningRef.current) return; // task finished — no reconnect needed
      if (reconnectTimerRef.current !== null) return; // already scheduled
      if (reconnectCountRef.current >= MAX_RECONNECT) {
        // Max retries exceeded — mark as failed
        setStatus('failed');
        setError(`连接丢失，已重试 ${MAX_RECONNECT} 次`);
        stopLocalTimer();
        isRunningRef.current = false;
        if (monitorCtx) {
          useTeamStore.getState().setActiveTeamTask({
            ...monitorCtx,
            status: 'error',
            error: `连接丢失，已重试 ${MAX_RECONNECT} 次`,
          });
        }
        return;
      }
      reconnectCountRef.current += 1;
      if (monitorCtx) {
        useTeamStore.getState().setActiveTeamTask({
          ...monitorCtx,
          status: 'running',
          partialOutput: accOutput ? `${accOutput}\n⟳ 正在重连...` : '⟳ 正在重连...',
        });
      }
      reconnectTimerRef.current = window.setTimeout(() => {
        reconnectTimerRef.current = null;
        if (isRunningRef.current) {
          connectWs(execId, monitorCtx);
        }
      }, 3000);
    };

    ws.onmessage = (event) => {
      hasReceivedMessage = true;
      reconnectCountRef.current = 0; // reset on successful message
      try {
        const data: AgentExecutionEvent = JSON.parse(event.data);
        switch (data.type) {
          case 'started':
            setStatus('started');
            if (data.role_id) {
              setActiveRoleId(data.role_id);
            }
            if (data.session_id) {
              setActiveSessionId(data.session_id);
            }
            break;
          case 'thinking':
            setStatus('thinking');
            if (data.elapsed_secs !== undefined) {
              setElapsedSecs(data.elapsed_secs);
            }
            if (monitorCtx && !accOutput) {
              useTeamStore.getState().setActiveTeamTask({
                ...monitorCtx,
                status: 'running',
                partialOutput: `AI 正在处理... (${data.elapsed_secs ?? 0}s)`,
              });
            }
            break;
          case 'output':
            if (data.partial_output) {
              setPartialOutput((prev) => prev + data.partial_output);
              if (monitorCtx) {
                accOutput += data.partial_output;
                useTeamStore.getState().setActiveTeamTask({
                  ...monitorCtx,
                  status: 'running',
                  partialOutput: accOutput,
                });
              }
            }
            break;
          case 'completed':
            setStatus('completed');
            setResult(data.result ?? null);
            setDurationMs(data.duration_ms ?? null);
            stopLocalTimer();
            isRunningRef.current = false;
            if (reconnectTimerRef.current !== null) {
              clearTimeout(reconnectTimerRef.current);
              reconnectTimerRef.current = null;
            }
            if (monitorCtx) {
              useTeamStore.getState().setActiveTeamTask({
                ...monitorCtx,
                status: 'done',
                result: data.result ?? '',
              });
            }
            break;
          case 'failed':
            setStatus('failed');
            setError(data.error ?? 'Unknown error');
            stopLocalTimer();
            isRunningRef.current = false;
            if (monitorCtx) {
              useTeamStore.getState().setActiveTeamTask({
                ...monitorCtx,
                status: 'error',
                error: data.error ?? 'Unknown error',
              });
            }
            break;
          case 'cancelled':
            setStatus('cancelled');
            stopLocalTimer();
            isRunningRef.current = false;
            if (monitorCtx) {
              useTeamStore.getState().setActiveTeamTask({
                ...monitorCtx,
                status: 'error',
                error: '已取消',
              });
            }
            break;
          case 'confirmation_required':
            setStatus('confirmation');
            setConfirmationQuestion(data.question ?? null);
            setConfirmationOptions(data.options ?? []);
            if (monitorCtx) {
              useTeamStore.getState().setActiveTeamTask({
                ...monitorCtx,
                status: 'waiting_confirmation',
                partialOutput: `${data.question ?? '需要确认'}\n选项: ${(data.options ?? []).join(', ')}`,
              });
            }
            break;
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onerror = () => {
      if (!isRunningRef.current) {
        setStatus((prev) =>
          prev === 'completed' || prev === 'failed' || prev === 'cancelled' ? prev : 'failed'
        );
        setError('WebSocket connection error');
        stopLocalTimer();
      } else if (hasReceivedMessage) {
        // Runtime error after successful connection — try reconnect
        scheduleReconnect();
      } else {
        // Initial connection error — retry with backoff
        scheduleReconnect();
      }
    };

    ws.onclose = () => {
      wsRef.current = null;
      // If task still running and connection dropped, schedule reconnect
      if (isRunningRef.current) {
        scheduleReconnect();
      }
    };
  }, [stopLocalTimer]);

  /** Execute a team task */
  const execute = useCallback(async (teamId: string, task: string, autoConfirm?: boolean): Promise<string | null> => {
    // Reset state
    setStatus('started');
    setElapsedSecs(0);
    setPartialOutput('');
    setResult(null);
    setError(null);
    setDurationMs(null);
    setActiveRoleId(null);
    setActiveSessionId(null);    startLocalTimer();
    isRunningRef.current = true;
    reconnectCountRef.current = 0;

    // 如果该团队开启了监控模式，更新全局悬浮卡
    const { teamMonitorMode, teams, setActiveTeamTask } = useTeamStore.getState();
    const isMonitor = teamMonitorMode[teamId] ?? false;
    // autoConfirm: 如果未指定，根据监控模式决定（监控模式 ON = 等待确认，OFF = 自动确认）
    const shouldAutoConfirm = autoConfirm !== undefined ? autoConfirm : !isMonitor;
    if (isMonitor) {
      const team = teams.find(t => t.id === teamId);
      setActiveTeamTask({
        teamId,
        teamName: team?.name ?? '团队',
        task,
        status: 'running',
      });
    }

    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/execute`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ team_id: teamId, task, context: {}, auto_confirm: shouldAutoConfirm }),
      });

      if (!response.ok) {
        if (isMonitor) {
          useTeamStore.getState().setActiveTeamTask({
            teamId,
            teamName: teams.find(t => t.id === teamId)?.name ?? '团队',
            task,
            status: 'error',
            error: `HTTP ${response.status}`,
          });
        }
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Backend returns immediately with execution_id embedded in final_output.
      // Actual result comes via WebSocket events (Completed/Failed).
      let execId: string | null = null;
      if (data.final_output) {
        try {
          const parsed = JSON.parse(data.final_output);
          execId = parsed.execution_id ?? null;
        } catch {
          // final_output is plain text — synchronous response
          execId = null;
        }
      }
      // Also check top-level execution_id (some routes return it directly)
      if (!execId) {
        execId = data.execution_id ?? null;
      }

      if (execId) {
        setExecutionId(execId);
        connectWs(execId, isMonitor ? { teamId, teamName: teams.find(t => t.id === teamId)?.name ?? '团队', task } : null);
        return execId;
      }

      // No execution_id — treat as synchronous completion
      setStatus('completed');
      isRunningRef.current = false;
      const finalOutput = data.final_output ?? '';
      setResult(finalOutput);
      stopLocalTimer();
      if (isMonitor) {
        useTeamStore.getState().setActiveTeamTask({
          teamId,
          teamName: teams.find(t => t.id === teamId)?.name ?? '团队',
          task,
          status: 'done',
          result: finalOutput,
        });
      }
      return null;
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : 'Unknown error';
      setStatus('failed');
      setError(errMsg);
      stopLocalTimer();
      isRunningRef.current = false;
      if (isMonitor) {
        useTeamStore.getState().setActiveTeamTask({
          teamId,
          teamName: teams.find(t => t.id === teamId)?.name ?? '团队',
          task,
          status: 'error',
          error: errMsg,
        });
      }
      return null;
    }
  }, [connectWs, startLocalTimer, stopLocalTimer]);

  /** Execute a group chat role turn */
  const executeRoleTurn = useCallback(async (sessionId: string, roleId: string): Promise<string | null> => {
    setStatus('started');
    setElapsedSecs(0);
    setPartialOutput('');
    setResult(null);
    setError(null);
    setDurationMs(null);
    setActiveRoleId(null);
    setActiveSessionId(null);    startLocalTimer();

    try {
      const response = await fetch(
        `${API_BASE}/api/v1/group-sessions/${sessionId}/execute-turn/${roleId}`,
        { method: 'POST' },
      );

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();
      const execId: string | null = data.execution_id ?? null;

      if (execId) {
        setExecutionId(execId);
        connectWs(execId);
        return execId;
      } else {
        setStatus('completed');
        setResult(JSON.stringify(data));
        stopLocalTimer();
        return null;
      }
    } catch (err) {
      setStatus('failed');
      setError(err instanceof Error ? err.message : 'Unknown error');
      stopLocalTimer();
      return null;
    }
  }, [connectWs, startLocalTimer, stopLocalTimer]);

  /** Cancel the current execution */
  const cancel = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'cancel' }));
    }
    wsRef.current?.close();
    setStatus('cancelled');
    stopLocalTimer();
  }, [stopLocalTimer]);

  /** Send confirmation response */
  const sendConfirmation = useCallback((response: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'confirm', response }));
    }
    setStatus('thinking');
    setConfirmationQuestion(null);
    setConfirmationOptions([]);
  }, []);

  /** Reset to idle */
  const reset = useCallback(() => {
    wsRef.current?.close();
    stopLocalTimer();
    setExecutionId(null);
    setStatus('idle');
    setElapsedSecs(0);
    setPartialOutput('');
    setResult(null);
    setError(null);
    setDurationMs(null);
    setActiveRoleId(null);
    setActiveSessionId(null);
  }, [stopLocalTimer]);

  return {
    executionId,
    status,
    elapsedSecs,
    partialOutput,
    result,
    error,
    durationMs,
    confirmationQuestion,
    confirmationOptions,
    activeRoleId,
    activeSessionId,
    execute,
    executeRoleTurn,
    sendConfirmation,
    cancel,
    reset,
  };
}
