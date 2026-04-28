import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import type { CreateWorkflowRequest } from '../api/client';
import { onWorkspaceChange } from './workspaceStore';

export interface Workflow {
  id: string;
  name: string;
  version: string;
  description?: string;
  stages: Stage[];
  agents: Agent[];
  triggers?: WorkflowTrigger[];
  // 列表摘要时由 API 返回，详情加载前作为展示用计数
  stage_count?: number;
  agent_count?: number;
  created_at?: string;
  updated_at?: string;
  workspace_id?: string;
}

export interface WorkflowTrigger {
  type: string;
  inputs?: Record<string, WorkflowInput>;
}

export interface WorkflowInput {
  type: string;
  required?: boolean;
  description?: string;
}

export interface Stage {
  name: string;
  stage_type?: string;
  agents: string[];
  parallel: boolean;
  question?: string;
  options?: Array<{ label: string; value: string; description?: string }>;
}

export interface Agent {
  id: string;
  role: string;
  model: string;
  prompt: string;
  depends_on: string[];
}

// API 返回的摘要类型
interface WorkflowSummary {
  id: string;
  name: string;
  version: string;
  description?: string;
  stage_count: number;
  agent_count: number;
}

// API_BASE_URL is imported from constants - use it directly

interface WorkflowStore {
  workflows: Workflow[];
  currentWorkflow: Workflow | null;
  loading: boolean;
  error: string | null;

  fetchWorkflows: () => Promise<void>;
  getWorkflow: (id: string) => Promise<Workflow | null>;
  createWorkflow: (workflow: CreateWorkflowRequest) => Promise<Workflow>;
  updateWorkflow: (id: string, workflow: Partial<Workflow>) => Promise<void>;
  deleteWorkflow: (id: string) => Promise<void>;
  setCurrentWorkflow: (workflow: Workflow | null) => void;
  clearError: () => void;
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

export const useWorkflowStore = create<WorkflowStore>((set) => ({
  workflows: [],
  currentWorkflow: null,
  loading: false,
  error: null,

  fetchWorkflows: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/workflows`, {}, 15000);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch workflows: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const summaries: WorkflowSummary[] = await response.json();

      if (summaries.length === 0) {
        set({ workflows: [], loading: false });
        return;
      }

      // 直接使用摘要数据，延迟加载完整详情
      // 只获取完整详情用于详情面板，而不是列表
      const basicWorkflows: Workflow[] = summaries.map((s) => ({
        id: s.id,
        name: s.name,
        version: s.version,
        description: s.description,
        stages: [],
        agents: [],
        stage_count: s.stage_count,
        agent_count: s.agent_count,
        created_at: undefined,
        updated_at: undefined,
      }));
      set({ workflows: basicWorkflows, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch workflows: ${message}`,
      });
    }
  },

  getWorkflow: async (id) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/workflows/${id}`, {}, 10000);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch workflow: ${response.status}`, response.status);
      }

      // 后端返回 { id, name, version, description, definition: { stages, agents, triggers, ... }, created_at, updated_at }
      // 需要把 definition 里的 stages/agents/triggers 提升到顶层
      const raw = await response.json();
      const def = raw.definition ?? {};
      return {
        id: raw.id,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        stages: def.stages ?? [],
        agents: def.agents ?? [],
        triggers: def.triggers ?? [],
        created_at: raw.created_at,
        updated_at: raw.updated_at,
      } as Workflow;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get workflow ${id}:`, message);
      return null;
    }
  },

  createWorkflow: async (workflow) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/workflows`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(workflow),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create workflow: ${response.status}`, response.status);
      }

      const newWorkflow = await response.json();
      set((state) => ({ workflows: [...state.workflows, newWorkflow] }));
      return newWorkflow;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create workflow: ${message}` });
      throw error; // 重新抛出，让调用者知道失败
    }
  },

  updateWorkflow: async (id, updates) => {
    // Optimistic update
    set((state) => ({
      workflows: state.workflows.map((w) =>
        w.id === id ? { ...w, ...updates, updated_at: new Date().toISOString() } : w,
      ),
    }));

    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/workflows/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to update workflow: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  deleteWorkflow: async (id) => {
    // 备份：失败时回滚（避免后端没删但 UI 已经删掉造成的"删除幻觉"）
    let prevState: { workflows: Workflow[]; currentWorkflow: Workflow | null } | null = null;

    // 乐观更新：先 UI 删掉，同时记下原状态
    set((state) => {
      prevState = {
        workflows: state.workflows,
        currentWorkflow: state.currentWorkflow,
      };
      return {
        workflows: state.workflows.filter((w) => w.id !== id),
        currentWorkflow: state.currentWorkflow?.id === id ? null : state.currentWorkflow,
      };
    });

    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/workflows/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to delete workflow: ${response.status}`, response.status);
      }
    } catch (error) {
      // 回滚 UI 状态，让用户看到的与后端实际一致
      if (prevState) {
        set(prevState);
      }
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `删除失败: ${message}` });
      throw error;
    }
  },

  setCurrentWorkflow: (workflow) => set({ currentWorkflow: workflow }),

  clearError: () => set({ error: null }),
}));
