import { create } from 'zustand';
import { API_BASE_URL, WS_BASE_URL } from '../api/constants';
import { unwrapEnvelope, fetchWithTimeout } from '../api/response';
import { maybeRecordFirstArtifact, recordRunCompleted } from '../services/factoryMetrics';

// WebSocket reconnection + poll fallback (AF-03)
const WS_RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 32000];
const WS_MAX_RECONNECT_ATTEMPTS = Infinity;
const WS_HEARTBEAT_INTERVAL = 30000;
const WS_POLL_INTERVAL_MS = 3000;
const WS_PING_MESSAGE = JSON.stringify({ type: 'ping' });

/** 单条 Run 的 WS/降级状态 */
export type WsConnectionStatus =
  | 'connected'
  | 'connecting'
  | 'disconnected'
  | 'reconnecting'
  | 'polling';

interface WsConnectionState {
  ws: WebSocket | null;
  status: WsConnectionStatus;
  reconnectAttempts: number;
  heartbeatTimer: ReturnType<typeof setInterval> | null;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
  pollTimer: ReturnType<typeof setInterval> | null;
}

// Track all WebSocket connections for cleanup
const allWsConnections = new Map<string, WebSocket>();

// Track WebSocket connection states (for reconnection and heartbeat)
const wsConnectionStates = new Map<string, WsConnectionState>();

// Cleanup all WebSocket connections (call on app unmount)
export function cleanupAllWebSockets() {
  allWsConnections.forEach((ws) => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close();
    }
  });
  allWsConnections.clear();

  // Clear all connection states
  wsConnectionStates.forEach((state) => {
    if (state.heartbeatTimer) clearInterval(state.heartbeatTimer);
    if (state.reconnectTimer) clearTimeout(state.reconnectTimer);
    if (state.pollTimer) clearInterval(state.pollTimer);
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
  team_id?: string;
  project_id?: string;
  trigger_source?: string;
  pending_pause?: WorkflowPauseState | null;
  approval_events?: ApprovalEvent[];
  /** 当前进行中的 stage（WS / poll 同步） */
  current_stage?: string;
}

export interface ApprovalEvent {
  stage_name: string;
  approved: boolean;
  comment?: string;
  decided_at: string;
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
  outputs?: StageOutput[];
  completed_at?: string;
  quality_gate_result?: QualityGateResult;
}

export interface StageOutput {
  path: string;
  content?: string;
  agent_id?: string;
  summary?: string;
  files_changed?: string[];
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
      pause_kind?: string;
    }
  | { type: 'workflow_resumed'; execution_id: string; stage_name: string; chosen_value: string }
  | { type: 'token_usage'; execution_id: string; total_tokens: number; total_cost_usd: number }
  | {
      type: 'snapshot';
      execution_id: string;
      status: string;
      current_stage?: string;
      output_log?: string[];
      stage_results?: StageResult[];
      pending_pause?: {
        stage_name: string;
        question: string;
        options: WorkflowPauseOption[];
        pause_kind?: string;
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
  pause_kind?: string;
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

interface ExecutionStore {
  executions: Execution[];
  currentExecution: Execution | null;
  loading: boolean;
  error: string | null;
  wsConnections: Map<string, WebSocket>;
  wsConnectionStatus: Map<string, WsConnectionStatus>;
  /** 每个 execution 的实时输出行（给 InlineExecPanel 读取） */
  outputLines: Map<string, RawLine[]>;
  /** 当前等待用户输入的暂停状态（null 表示没有暂停） */
  pendingPause: WorkflowPauseState | null;

  fetchExecutions: () => Promise<void>;
  getExecution: (id: string) => Promise<Execution | null>;
  startExecution: (workflowId: string, variables?: Record<string, unknown>) => Promise<Execution>;
  cancelExecution: (id: string) => Promise<{ ok: boolean; error?: string }>;
  deleteExecutions: (ids: string[]) => Promise<void>;
  resumeExecution: (executionId: string, value: string) => boolean;
  resolveExecution: (
    executionId: string,
    approved: boolean,
    comment?: string,
  ) => Promise<void>;
  setCurrentExecution: (execution: Execution | null) => void;
  connectWebSocket: (executionId: string) => void;
  disconnectWebSocket: (executionId: string) => void;
  clearError: () => void;
  dismissPause: () => void;
}

/** 列表 API 不含 stage_results；合并时保留 WS/详情已拉取的字段 */
function mergeExecutionList(incoming: Execution[], existing: Execution[]): Execution[] {
  return incoming.map((item) => {
    const prev = existing.find((e) => e.id === item.id);
    if (!prev) return item;
    const prevStages = prev.stage_results?.length ?? 0;
    const nextStages = item.stage_results?.length ?? 0;
    return {
      ...item,
      stage_results:
        prevStages > nextStages ? prev.stage_results : (item.stage_results ?? prev.stage_results),
      current_stage: prev.current_stage ?? item.current_stage,
      pending_pause: prev.pending_pause ?? item.pending_pause,
      resumed_from: prev.resumed_from ?? item.resumed_from,
      variables:
        prev.variables && Object.keys(prev.variables).length > 0
          ? prev.variables
          : item.variables,
    };
  });
}

function upsertExecutionInStore(
  set: (partial: Partial<ExecutionStore> | ((state: ExecutionStore) => Partial<ExecutionStore>)) => void,
  exec: Execution,
) {
  set((state) => {
    const idx = state.executions.findIndex((e) => e.id === exec.id);
    if (idx < 0) return { executions: [...state.executions, exec] };
    const executions = [...state.executions];
    executions[idx] = { ...executions[idx], ...exec };
    return { executions };
  });
  if (exec.status === 'paused' && exec.pending_pause) {
    set({
      pendingPause: {
        execution_id: exec.id,
        stage_name: exec.pending_pause.stage_name,
        question: exec.pending_pause.question,
        options: exec.pending_pause.options,
        pause_kind: exec.pending_pause.pause_kind,
      },
    });
  }
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

      const data = unwrapEnvelope<Execution[]>(await response.json());
      set((state) => ({
        executions: mergeExecutionList(data, state.executions),
        loading: false,
      }));
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

      const exec = unwrapEnvelope<Execution>(await response.json());
      upsertExecutionInStore(set, exec);
      return exec;
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

      const execution = unwrapEnvelope<Execution>(await response.json());

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
    const prev = get().executions;
    const now = new Date().toISOString();

    set((state) => ({
      executions: state.executions.map((e) =>
        e.id === id ? { ...e, status: 'cancelled' as const, finished_at: now } : e,
      ),
    }));

    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions/${id}/cancel`, {
        method: 'POST',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to cancel execution: ${response.status}`, response.status);
      }

      const result = await response.json();
      if (!result.success) {
        set({ executions: prev });
        const error = result.message || '取消失败，执行可能已结束';
        void get().fetchExecutions();
        return { ok: false, error };
      }

      get().disconnectWebSocket(id);
      void get().fetchExecutions();
      return { ok: true };
    } catch (error) {
      set({ executions: prev });
      const message = error instanceof Error ? error.message : 'Unknown error';
      return { ok: false, error: `取消失败: ${message}` };
    }
  },

  setCurrentExecution: (execution) => set({ currentExecution: execution }),

  deleteExecutions: async (ids) => {
    if (ids.length === 0) return;
    // Optimistic: 前端先移除
    const prev = get().executions;
    set({ executions: prev.filter((e) => !ids.includes(e.id)) });

    // 先断开相关 WS 连接（避免幽灵连接）
    ids.forEach((id) => get().disconnectWebSocket(id));

    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/executions/batch-delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ids }),
      });
      if (!response.ok) {
        throw new ApiError(`批量删除失败: ${response.status}`, response.status);
      }
    } catch (error) {
      // 回滚
      set({ executions: prev });
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `批量删除失败: ${message}` });
      throw error;
    }
  },

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

  resolveExecution: async (executionId, approved, comment) => {
    const response = await fetchWithTimeout(
      `${API_BASE_URL}/api/v1/executions/${executionId}/resolve`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ approved, comment }),
      },
    );
    if (!response.ok) {
      let message = `审批失败: ${response.status}`;
      try {
        const body = await response.json();
        if (body?.error) message = body.error;
      } catch {
        // ignore
      }
      throw new ApiError(message, response.status);
    }
    set((state) => ({
      pendingPause:
        state.pendingPause?.execution_id === executionId ? null : state.pendingPause,
      executions: state.executions.map((e) =>
        e.id === executionId
          ? {
              ...e,
              status: 'running' as const,
              pending_pause: null,
            }
          : e,
      ),
    }));
    get().connectWebSocket(executionId);
  },

  dismissPause: () => set({ pendingPause: null }),

  connectWebSocket: (executionId) => {
    const { wsConnections } = get();
    const existing = wsConnectionStates.get(executionId);
    if (
      existing &&
      (existing.status === 'connected' ||
        existing.status === 'connecting' ||
        existing.status === 'polling' ||
        existing.status === 'reconnecting')
    ) {
      return;
    }
    if (wsConnections.has(executionId) && existing?.status === 'connected') return;

    const connectionState: WsConnectionState = existing ?? {
      ws: null,
      status: 'connecting',
      reconnectAttempts: 0,
      heartbeatTimer: null,
      reconnectTimer: null,
      pollTimer: null,
    };
    connectionState.status = 'connecting';
    wsConnectionStates.set(executionId, connectionState);
    set((state) => ({
      wsConnectionStatus: new Map(state.wsConnectionStatus).set(executionId, 'connecting'),
    }));

    const setExecWsStatus = (execId: string, status: WsConnectionStatus) => {
      const st = wsConnectionStates.get(execId);
      if (st) st.status = status;
      set((s) => ({
        wsConnectionStatus: new Map(s.wsConnectionStatus).set(execId, status),
      }));
    };

    const mergeExecutionDetail = (exec: Execution) => {
      set((state) => {
        const idx = state.executions.findIndex((e) => e.id === exec.id);
        if (idx < 0) return { executions: [...state.executions, exec] };
        const executions = [...state.executions];
        executions[idx] = { ...executions[idx], ...exec };
        return { executions };
      });
      if (exec.status === 'paused' && exec.pending_pause) {
        set({
          pendingPause: {
            execution_id: exec.id,
            stage_name: exec.pending_pause.stage_name,
            question: exec.pending_pause.question,
            options: exec.pending_pause.options,
            pause_kind: exec.pending_pause.pause_kind,
          },
        });
      }
    };

    const stopPollFallback = (execId: string) => {
      const st = wsConnectionStates.get(execId);
      if (!st?.pollTimer) return;
      clearInterval(st.pollTimer);
      st.pollTimer = null;
    };

    const pollOnce = async (execId: string) => {
      const exec = await get().getExecution(execId);
      if (!exec) return;
      mergeExecutionDetail(exec);
      if (exec.status === 'completed' || exec.status === 'failed' || exec.status === 'cancelled') {
        stopPollFallback(execId);
        cleanupConnection(execId);
      }
    };

    const startPollFallback = (execId: string) => {
      const st = wsConnectionStates.get(execId);
      if (!st || st.pollTimer) return;
      setExecWsStatus(execId, 'polling');
      void pollOnce(execId);
      st.pollTimer = setInterval(() => void pollOnce(execId), WS_POLL_INTERVAL_MS);
    };

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
          stopPollFallback(executionId);
          state.status = 'connected';
          state.reconnectAttempts = 0;
          setExecWsStatus(executionId, 'connected');
          startHeartbeat(executionId);
        };

        ws.onclose = () => {
          if (state.heartbeatTimer) {
            clearInterval(state.heartbeatTimer);
            state.heartbeatTimer = null;
          }

          const currentExec = get().executions.find((e) => e.id === executionId);
          if (
            currentExec?.status === 'completed' ||
            currentExec?.status === 'failed' ||
            currentExec?.status === 'cancelled'
          ) {
            stopPollFallback(executionId);
            cleanupConnection(executionId);
            return;
          }

          state.status = 'disconnected';
          startPollFallback(executionId);
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
        startPollFallback(executionId);
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

      setExecWsStatus(execId, 'reconnecting');

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
        if (state.pollTimer) clearInterval(state.pollTimer);
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
        set((state) => {
          const executions = [...state.executions];
          const idx = executions.findIndex((e) => e.id === event.execution_id);
          if (idx >= 0) {
            const updated = { ...executions[idx] };
            if (event.status) {
              updated.status = event.status as Execution['status'];
            }
            if (event.stage_results) {
              updated.stage_results = event.stage_results;
            }
            if (event.current_stage) {
              updated.current_stage = event.current_stage;
            }
            executions[idx] = updated;
          }
          return { executions };
        });
        if (event.pending_pause) {
          set({
            pendingPause: {
              execution_id: event.execution_id,
              stage_name: event.pending_pause.stage_name,
              question: event.pending_pause.question,
              options: event.pending_pause.options,
              pause_kind: event.pending_pause.pause_kind,
            },
          });
        }
        if (event.output_log && event.output_log.length > 0) {
          set((state) => {
            const map = new Map(state.outputLines);
            const existing = map.get(event.execution_id) ?? [];
            if (existing.length === 0) {
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
            pause_kind: event.pause_kind,
          },
          executions: state.executions.map((e) =>
            e.id === event.execution_id
              ? {
                  ...e,
                  status: 'paused' as const,
                  pending_pause: {
                    execution_id: event.execution_id,
                    stage_name: event.stage_name,
                    question: event.question,
                    options: event.options,
                    pause_kind: event.pause_kind,
                  },
                }
              : e,
          ),
        }));
        return;
      }

      if (event.type === 'workflow_resumed') {
        set((state) => ({
          pendingPause:
            state.pendingPause?.execution_id === event.execution_id ? null : state.pendingPause,
          executions: state.executions.map((e) =>
            e.id === event.execution_id
              ? { ...e, status: 'running' as const, pending_pause: null }
              : e,
          ),
        }));
        return;
      }

      set((state) => {
        const executions = [...state.executions];
        const idx = executions.findIndex((e) => e.id === event.execution_id);

        switch (event.type) {
          case 'stage_started':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                current_stage: event.stage_name,
                status: 'running',
              };
            }
            break;
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
                outputs: [event.output as StageOutput],
                completed_at: new Date().toISOString(),
                quality_gate_result: (
                  event as unknown as { quality_gate_result?: QualityGateResult }
                ).quality_gate_result,
              });
              executions[idx] = {
                ...executions[idx],
                stage_results: stageResults,
                current_stage: undefined,
              };
              void maybeRecordFirstArtifact(event.execution_id);
            }
            break;
          case 'completed':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                status: 'completed',
                finished_at: new Date().toISOString(),
                current_stage: undefined,
              };
            }
            void recordRunCompleted(event.execution_id, 'completed');
            stopPollFallback(event.execution_id);
            break;
          case 'failed':
            if (idx >= 0) {
              executions[idx] = {
                ...executions[idx],
                status: 'failed',
                error: event.error,
                finished_at: new Date().toISOString(),
                current_stage: undefined,
              };
            }
            void recordRunCompleted(event.execution_id, 'failed');
            stopPollFallback(event.execution_id);
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
      if (state.pollTimer) clearInterval(state.pollTimer);
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
