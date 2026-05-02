import { unwrapEnvelope } from '../api/response';
import { create } from 'zustand';
import { API_BASE_URL, WS_BASE_URL } from '../api/constants';
import { unwrapEnvelope, fetchWithTimeout } from '../api/response';

// WebSocket reconnection config
const WS_RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 32000]; // exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s (max)
const WS_MAX_RECONNECT_ATTEMPTS = Infinity; // configurable, Infinity means unlimited
const WS_HEARTBEAT_INTERVAL = 30000; // 30 seconds
const WS_PING_MESSAGE = JSON.stringify({ type: 'ping' });

// Connection status type
type ConnectionStatus = 'connected' | 'connecting' | 'disconnected';

// WebSocket connection state per execution
interface WsConnectionState {
  ws: WebSocket | null;
  status: ConnectionStatus;
  reconnectAttempts: number;
  heartbeatTimer: ReturnType<typeof setInterval> | null;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
}

// Track all WebSocket connections for cleanup
const allWsConnections = new Map<string, WebSocket>();

// Track WebSocket connection states (for reconnection and heartbeat)
const wsConnectionStates = new Map<string, WsConnectionState>();

// Cleanup all WebSocket connections (call on app unmount)
export function cleanupAllWebSockets() {
  allWsConnections.forEach((ws, id) => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close();
    }
  });
  allWsConnections.clear();

  // Clear all connection states
  wsConnectionStates.forEach((state) => {
    if (state.heartbeatTimer) clearInterval(state.heartbeatTimer);
    if (state.reconnectTimer) clearTimeout(state.reconnectTimer);
  });
  wsConnectionStates.clear();
}

export interface Execution {
  id: string;
  workflow_id: string;
  status: 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled' | 'interrupted';
  resumed_from?: string;
  variables?: Record<string, unknown>;
  stage_results?: StageResult[];
  started_at?: string;
  finished_at?: string;
  error?: string;
  total_tokens?: number;
  total_cost_usd?: number;
}

export interface QualityCheckResult {
  cmd: string;
  passed: boolean;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
}

export interface QualityGateResult {
  passed: boolean;
  checks: QualityCheckResult[];
  retry_count: number;
}

export interface StageResult {
  stage_name: string;
  outputs?: unknown[];
  completed_at?: string;
  quality_gate_result?: QualityGateResult;
}

/** 实时输出行，供 InlineExecPanel 消费 */
export interface RawLine {
  id: number;
  type: 'info' | 'output' | 'stage_started' | 'stage_completed' | 'completed' | 'error';
  content: string;
  stageName?: string;
}

// 每个 execution 的行计数器（模块级，不放进 store 避免序列化）
const lineCounters = new Map<string, number>();
function nextLineId(execId: string): number {
  const n = (lineCounters.get(execId) ?? 0) + 1;
  lineCounters.set(execId, n);
  return n;
}

// 执行事件类型
type ExecutionEvent =
  | { type: 'started'; execution_id: string; workflow_id: string }
  | { type: 'status_changed'; execution_id: string; status: string }
  | { type: 'stage_started'; execution_id: string; stage_name: string }
  | { type: 'stage_completed'; execution_id: string; stage_name: string; output: unknown }
  | { type: 'output'; execution_id: string; line: string }
  | { type: 'completed'; execution_id: string }
  | { type: 'failed'; execution_id: string; error: string }
  | {
      type: 'workflow_paused';
      execution_id: string;
      stage_name: string;
      question: string;
      options: WorkflowPauseOption[];
    }
  | { type: 'workflow_resumed'; execution_id: string; stage_name: string; chosen_value: string }
  | { type: 'token_usage'; execution_id: string; total_tokens: number; total_cost_usd: number }
  | {
      type: 'snapshot';
      execution_id: string;
      status: string;
      current_stage?: string;
      output_log?: string[];
      pending_pause?: {
        stage_name: string;
        question: string;
        options: WorkflowPauseOption[];
      } | null;
    }
  | { type: 'pong' }; // heartbeat response

export interface WorkflowPauseOption {
  label: string;
  value: string;
}

export interface WorkflowPauseState {
  execution_id: string;
  stage_name: string;
  question: string;
  options: WorkflowPauseOption[];
}

// 自定义错误类型
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

// 带 timeout 的 fetch
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 5000,
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

interface ExecutionStore {
  executions: Execution[];
  currentExecution: Execution | null;
  loading: boolean;
  error: string | null;
  wsConnections: Map<string, WebSocket>;
  wsConnectionStatus: Map<string, ConnectionStatus>;
  /** 每个 execution 的实时输出行（给 InlineExecPanel 读取） */
  outputLines: Map<string, RawLine[]>;
  /** 当前等待用户输入的暂停状态（null 表示没有暂停） */
  pendingPause: WorkflowPauseState | null;

  fetchExecutions: () => Promise<void>;
  getExecution: (id: string) => Promise<Execution | null>;
  startExecution: (workflowId: string, variables?: Record<string, unknown>) => Promise<Execution>;
  cancelExecution: (id: string) => Promise<void>;
  resumeExecution: (executionId: string, value: string) => boolean;
  setCurrentExecution: (execution: Execution | null) => void;
  connectWebSocket: (executionId: string) => void;
  disconnectWebSocket: (executionId: string) => void;
  clearError: () => void;
  dismissPause: () => void;
}

export const useExecutionStore = create<ExecutionStore>((set, get) => ({
  executions: [],
  currentExecution: null,
  loading: false,
  error: null,
  wsConnections: new Map(),
  wsConnectionStatus: new Map(),
  outputLines: new Map(),
  pendingPause: null,

  fetchExecutions: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions`);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch executions: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const data = unwrapEnvelope(await response.json());
      set({ executions: data, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch executions: ${message}`,
      });
    }
  },

  getExecution: async (id) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions/${id}`);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch execution: ${response.status}`, response.status);
      }

      return unwrapEnvelope(await response.json());
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get execution ${id}:`, message);
      return null;
    }
  },

  startExecution: async (workflowId, variables = {}) => {
    set({ error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ workflow_id: workflowId, variables }),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to start execution: ${response.status}`, response.status);
      }

      const execution = unwrapEnvelope(await response.json());

      set((state) => ({ executions: [...state.executions, execution] }));

      // 连接 WebSocket 获取实时更新
      get().connectWebSocket(execution.id);

      return execution;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to start execution: ${message}` });
      throw error;
    }
  },

  cancelExecution: async (id) => {
    // Optimistic update
    set((state) => ({
      executions: state.executions.map((e) =>
        e.id === id ? { ...e, status: 'cancelled' as const } : e,
      ),
    }));

    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions/${id}/cancel`, {
        method: 'POST',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to cancel execution: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  setCurrentExecution: (execution) => set({ currentExecution: execution }),

  resumeExecution: (executionId, value) => {
    const { wsConnections } = get();
    const ws = wsConnections.get(executionId);
    const canSend = ws !== undefined && ws.readyState === WebSocket.OPEN;
    if (canSend) {
      ws!.send(JSON.stringify({ type: 'resume_workflow', execution_id: executionId, value }));
      set((state) => ({
        pendingPause: state.pendingPause?.execution_id === executionId ? null : state.pendingPause,
        executions: state.executions.map((e) =>
          e.id === executionId ? { ...e, status: 'running' as const } : e,
        ),
      }));
    }
    return canSend;
  },

  dismissPause: () => set({ pendingPause: null }),

  connectWebSocket: (executionId) => {
    const { wsConnections, wsConnectionStatus } = get();
    if (wsConnections.has(executionId)) return;

    // Initialize connection state
    const connectionState: WsConnectionState = {
      ws: null,
      status: 'connecting',
      reconnectAttempts: 0,
      heartbeatTimer: null,
      reconnectTimer: null,
    };
    wsConnectionStates.set(executionId, connectionState);
    set((state) => ({
      wsConnectionStatus: new Map(state.wsConnectionStatus).set(executionId, 'connecting'),
    }));

    const connect = () => {
      const state = wsConnectionStates.get(executionId);
      if (!state || state.status === 'connected') return;

      try {
        const ws = new WebSocket(`${WS_BASE_URL}/ws/executions/${executionId}`);
        state.ws = ws;

        set((s) => ({
          wsConnections: new Map(s.wsConnections).set(executionId, ws),
          wsConnectionStatus: new Map(s.wsConnectionStatus).set(executionId, 'connecting'),
        }));

        ws.onmessage = (event) => {
          try {
            const data: ExecutionEvent = JSON.parse(event.data);
            // Handle pong heartbeat response - reset heartbeat timer
            if (data.type === 'pong') {
              return;
            }
            handleExecutionEvent(data);
          } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
          }
        };

        ws.onopen = () => {
          state.status = 'connected';
          state.reconnectAttempts = 0;
          set((s) => ({
            wsConnectionStatus: new Map(s.wsConnectionStatus).set(executionId, 'connected'),
          }));
          // Start heartbeat
          startHeartbeat(executionId);
        };

        ws.onclose = () => {
          // Clear heartbeat
          if (state.heartbeatTimer) {
            clearInterval(state.heartbeatTimer);
            state.heartbeatTimer = null;
          }

          // Check if execution is completed/failed - if so, don't reconnect
          const currentExec = get().executions.find((e) => e.id === executionId);
          if (
            currentExec?.status === 'completed' ||
            currentExec?.status === 'failed' ||
            currentExec?.status === 'cancelled'
          ) {
            cleanupConnection(executionId);
            return;
          }

          // Attempt reconnection with exponential backoff
          state.status = 'disconnected';
          set((s) => ({
            wsConnectionStatus: new Map(s.wsConnectionStatus).set(executionId, 'disconnected'),
          }));
          scheduleReconnect(executionId);
        };

        ws.onerror = () => {
          // Error will trigger onclose, so handle there
          ws.close();
        };

        allWsConnections.set(executionId, ws);
      } catch (error) {
        console.error('WebSocket connection failed:', error);
        state.status = 'disconnected';
        set((s) => ({
          wsConnectionStatus: new Map(s.wsConnectionStatus).set(executionId, 'disconnected'),
        }));
        scheduleReconnect(executionId);
      }
    };

    const startHeartbeat = (execId: string) => {
      const state = wsConnectionStates.get(execId);
      if (!state) return;

      // Clear existing heartbeat
      if (state.heartbeatTimer) {
        clearInterval(state.heartbeatTimer);
      }

      state.heartbeatTimer = setInterval(() => {
        const s = wsConnectionStates.get(execId);
        if (s?.ws && s.ws.readyState === WebSocket.OPEN) {
          s.ws.send(WS_PING_MESSAGE);
        }
      }, WS_HEARTBEAT_INTERVAL);
    };

    const scheduleReconnect = (execId: string) => {
      const state = wsConnectionStates.get(execId);
      if (!state) return;

      // Check max attempts
      if (state.reconnectAttempts >= WS_MAX_RECONNECT_ATTEMPTS) {
        console.warn(`WebSocket reconnection limit reached for execution ${execId}`);
        return;
      }

      // Calculate delay with exponential backoff
      const delayIndex = Math.min(state.reconnectAttempts, WS_RECONNECT_DELAYS.length - 1);
      const delay = WS_RECONNECT_DELAYS[delayIndex];

      state.reconnectAttempts++;

      state.reconnectTimer = setTimeout(() => {
        const s = wsConnectionStates.get(execId);
        if (s && s.status !== 'connected') {
          connect();
        }
      }, delay);
    };

    const cleanupConnection = (execId: string) => {
      const state = wsConnectionStates.get(execId);
      if (state) {
        if (state.heartbeatTimer) clearInterval(state.heartbeatTimer);
        if (state.reconnectTimer) clearTimeout(state.reconnectTimer);
        wsConnectionStates.delete(execId);
      }

      set((s) => {
        const newWsConnections = new Map(s.wsConnections);
        newWsConnections.delete(execId);
        const newStatus = new Map(s.wsConnectionStatus);
        newStatus.delete(execId);
        return { wsConnections: newWsConnections, wsConnectionStatus: newStatus };
      });
      allWsConnections.delete(execId);
    };

    function appendLine(execId: string, line: Omit<RawLine, 'id'>) {
      set((state) => {
        const map = new Map(state.outputLines);
        const existing = map.get(execId) ?? [];
        map.set(execId, [...existing, { ...line, id: nextLineId(execId) }]);
        return { outputLines: map };
      });
    }

    function handleExecutionEvent(event: ExecutionEvent) {
      // Ignore pong messages - they don't have execution_id
      if (event.type === 'pong') {
        return;
      }

      // Handle snapshot (catch-up when connecting to already-running execution)
      if (event.type === 'snapshot') {
        if (event.pending_pause) {
          set({
            pendingPause: {
              execution_id: event.execution_id,
              stage_name: event.pending_pause.stage_name,
              question: event.pending_pause.question,
              options: event.pending_pause.options,
            },
          });
        }
        if (event.output_log && event.output_log.length > 0) {
          set((state) => {
            const map = new Map(state.outputLines);
            const existing = map.get(event.execution_id) ?? [];
            if (existing.length === 0) {
              // Only restore from snapshot if no lines yet (avoid duplicates)
              const lines = event.output_log!.map((content) => ({
                id: nextLineId(event.execution_id),
                type: 'output' as const,
                content,
              }));
              map.set(event.execution_id, lines);
            }
            return { outputLines: map };
          });
        }
        return;
      }

      // ── 实时输出行累积（供 InlineExecPanel 读取）──
      switch (event.type) {
        case 'started':
          appendLine(event.execution_id, { type: 'info', content: '工作流已启动' });
          break;
        case 'stage_started':
          appendLine(event.execution_id, {
            type: 'stage_started',
            content: '',
            stageName: event.stage_name,
          });
          break;
        case 'output':
          appendLine(event.execution_id, { type: 'output', content: event.line });
          break;
        case 'stage_completed':
          appendLine(event.execution_id, {
            type: 'stage_completed',
            content: '',
            stageName: event.stage_name,
          });
          break;
        case 'completed':
          appendLine(event.execution_id, { type: 'completed', content: '工作流执行完成 ✓' });
          break;
        case 'failed':
          appendLine(event.execution_id, { type: 'error', content: `执行失败: ${event.error}` });
          break;
      }

      // Handle workflow_paused outside of executions array update
      if (event.type === 'workflow_paused') {
        set((state) => ({
          pendingPause: {
            execution_id: event.execution_id,
            stage_name: event.stage_name,
            question: event.question,
            options: event.options,
          },
          executions: state.executions.map((e) =>
            e.id === event.execution_id ? { ...e, status: 'running' as const } : e,
          ),
        }));
        return;
      }

      if (event.type === 'workflow_resumed') {
        // Clear pause state if it matches
        set((state) => ({
          pendingPause:
            state.pendingPause?.execution_id === event.execution_id ? null : state.pendingPause,
        }));
        return;
      }

      set((state) => {
        const executions = [...state.executions];
        const idx = executions.findIndex((e) => e.id === event.execution_id);

        switch (event.type) {
          case 'status_changed':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                status: event.status as Execution['status'],
              };
            }
            break;
          case 'stage_completed':
            if (idx >= 0) {
              const stageResults = [...(executions[idx].stage_results || [])];
              stageResults.push({
                stage_name: event.stage_name,
                outputs: [event.output],
                completed_at: new Date().toISOString(),
                quality_gate_result: event.quality_gate_result as QualityGateResult | undefined,
              });
              executions[idx] = { ...executions[idx], stage_results: stageResults };
            }
            break;
          case 'completed':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                status: 'completed',
                finished_at: new Date().toISOString(),
              };
            }
            break;
          case 'failed':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                status: 'failed',
                error: event.error,
                finished_at: new Date().toISOString(),
              };
            }
            break;
          case 'token_usage':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                total_tokens: event.total_tokens,
                total_cost_usd: event.total_cost_usd,
              };
            }
            break;
        }

        return { executions };
      });
    }

    // Start the connection
    connect();
  },

  disconnectWebSocket: (executionId) => {
    const { wsConnections } = get();
    const ws = wsConnections.get(executionId);
    if (ws) {
      ws.close();
    }

    // Clean up connection state
    const state = wsConnectionStates.get(executionId);
    if (state) {
      if (state.heartbeatTimer) clearInterval(state.heartbeatTimer);
      if (state.reconnectTimer) clearTimeout(state.reconnectTimer);
      wsConnectionStates.delete(executionId);
    }

    allWsConnections.delete(executionId);

    // Use set() to properly trigger state update with new Map reference
    set((s) => {
      const newWsConnections = new Map(s.wsConnections);
      newWsConnections.delete(executionId);
      const newStatus = new Map(s.wsConnectionStatus);
      newStatus.delete(executionId);
      return { wsConnections: newWsConnections, wsConnectionStatus: newStatus };
    });
  },

  clearError: () => set({ error: null }),
}));
