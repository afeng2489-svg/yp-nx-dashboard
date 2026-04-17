import { useState, useRef, useCallback, useEffect } from 'react';
import { WS_BASE_URL } from '@/api/constants';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

/** Agent execution event from WebSocket */
interface AgentExecutionEvent {
  type: 'started' | 'thinking' | 'output' | 'completed' | 'failed' | 'cancelled';
  execution_id: string;
  agent_role?: string;
  task_summary?: string;
  elapsed_secs?: number;
  partial_output?: string;
  result?: string;
  duration_ms?: number;
  error?: string;
}

export type AgentExecutionStatus =
  | 'idle'
  | 'started'
  | 'thinking'
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
  execute: (teamId: string, task: string) => Promise<string | null>;
  executeRoleTurn: (sessionId: string, roleId: string) => Promise<string | null>;
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

  const wsRef = useRef<WebSocket | null>(null);
  const timerRef = useRef<number | null>(null);
  const startTimeRef = useRef<number>(0);

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

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopLocalTimer();
      wsRef.current?.close();
    };
  }, [stopLocalTimer]);

  /** Connect to WS and listen for events */
  const connectWs = useCallback((execId: string) => {
    const wsUrl = `${WS_BASE_URL}/ws/agent-executions/${execId}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      try {
        const data: AgentExecutionEvent = JSON.parse(event.data);
        switch (data.type) {
          case 'started':
            setStatus('started');
            break;
          case 'thinking':
            setStatus('thinking');
            if (data.elapsed_secs !== undefined) {
              setElapsedSecs(data.elapsed_secs);
            }
            break;
          case 'output':
            if (data.partial_output) {
              setPartialOutput((prev) => prev + data.partial_output);
            }
            break;
          case 'completed':
            setStatus('completed');
            setResult(data.result ?? null);
            setDurationMs(data.duration_ms ?? null);
            stopLocalTimer();
            break;
          case 'failed':
            setStatus('failed');
            setError(data.error ?? 'Unknown error');
            stopLocalTimer();
            break;
          case 'cancelled':
            setStatus('cancelled');
            stopLocalTimer();
            break;
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onerror = () => {
      // WS error — don't overwrite existing status if already terminal
      setStatus((prev) =>
        prev === 'completed' || prev === 'failed' || prev === 'cancelled'
          ? prev
          : 'failed'
      );
      setError('WebSocket connection error');
      stopLocalTimer();
    };

    ws.onclose = () => {
      wsRef.current = null;
    };
  }, [stopLocalTimer]);

  /** Execute a team task */
  const execute = useCallback(async (teamId: string, task: string): Promise<string | null> => {
    // Reset state
    setStatus('started');
    setElapsedSecs(0);
    setPartialOutput('');
    setResult(null);
    setError(null);
    setDurationMs(null);
    startLocalTimer();

    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/execute`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ team_id: teamId, task, context: {} }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Parse execution_id from response
      let execId: string | null = null;
      if (data.final_output) {
        try {
          const parsed = JSON.parse(data.final_output);
          execId = parsed.execution_id ?? null;
        } catch {
          // final_output is not JSON — this means synchronous response (shouldn't happen now)
          execId = null;
        }
      }

      if (execId) {
        setExecutionId(execId);
        connectWs(execId);
        return execId;
      } else {
        // Fallback: treat as synchronous completion
        setStatus('completed');
        setResult(data.final_output ?? '');
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

  /** Execute a group chat role turn */
  const executeRoleTurn = useCallback(async (sessionId: string, roleId: string): Promise<string | null> => {
    setStatus('started');
    setElapsedSecs(0);
    setPartialOutput('');
    setResult(null);
    setError(null);
    setDurationMs(null);
    startLocalTimer();

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
  }, [stopLocalTimer]);

  return {
    executionId,
    status,
    elapsedSecs,
    partialOutput,
    result,
    error,
    durationMs,
    execute,
    executeRoleTurn,
    cancel,
    reset,
  };
}
