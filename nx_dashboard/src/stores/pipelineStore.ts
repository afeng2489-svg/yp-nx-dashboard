import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

// --- Types ---

export type StepStatus =
  | 'pending'
  | 'ready'
  | 'running'
  | 'completed'
  | 'failed'
  | 'skipped'
  | 'blocked';
export type PipelineStatusType = 'idle' | 'running' | 'paused' | 'completed' | 'failed';

export interface PipelineStep {
  id: string;
  task_id: string;
  phase: string;
  role_id: string;
  instruction: string;
  status: StepStatus;
  output?: string;
  retry_count: number;
}

export interface ProgressSummary {
  total_steps: number;
  completed_steps: number;
  running_steps: number;
  failed_steps: number;
  progress_pct: number;
}

export interface PipelineData {
  id: string;
  project_id: string;
  team_id: string;
  current_phase: string;
  status: PipelineStatusType;
  steps: PipelineStep[];
  progress: ProgressSummary;
}

interface PipelineState {
  pipeline: PipelineData | null;
  loading: boolean;
  error: string | null;

  fetchPipeline: (projectId: string) => Promise<void>;
  createPipeline: (projectId: string, teamId: string) => Promise<void>;
  startPipeline: (pipelineId: string) => Promise<void>;
  pausePipeline: (pipelineId: string) => Promise<void>;
  resumePipeline: (pipelineId: string) => Promise<void>;
  retryStep: (pipelineId: string, stepId: string) => Promise<void>;
  clearError: () => void;
  reset: () => void;
}

// --- Store ---

export const usePipelineStore = create<PipelineState>((set, get) => ({
  pipeline: null,
  loading: false,
  error: null,

  fetchPipeline: async (projectId: string) => {
    set({ loading: true, error: null, pipeline: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/projects/${projectId}/pipeline`);
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ pipeline: data, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false, pipeline: null });
    }
  },

  createPipeline: async (projectId: string, teamId: string) => {
    set({ loading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/projects/${projectId}/pipeline`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ team_id: teamId }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ pipeline: data, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  startPipeline: async (pipelineId: string) => {
    set({ loading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/pipelines/${pipelineId}/start`, {
        method: 'POST',
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ pipeline: data, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  pausePipeline: async (pipelineId: string) => {
    set({ loading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/pipelines/${pipelineId}/pause`, {
        method: 'POST',
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ pipeline: data, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  resumePipeline: async (pipelineId: string) => {
    set({ loading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/pipelines/${pipelineId}/resume`, {
        method: 'POST',
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ pipeline: data, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  retryStep: async (pipelineId: string, stepId: string) => {
    set({ error: null });
    try {
      const res = await fetch(
        `${API_BASE_URL}/api/v1/pipelines/${pipelineId}/steps/${stepId}/retry`,
        { method: 'POST' },
      );
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      // Refresh full pipeline status after retry
      const current = get().pipeline;
      if (current) {
        const refreshRes = await fetch(`${API_BASE_URL}/api/v1/pipelines/${pipelineId}/steps`);
        if (refreshRes.ok) {
          const data = await refreshRes.json();
          // /steps returns PipelineResponse which has the same shape as PipelineData
          set({ pipeline: data });
        }
      }
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  clearError: () => set({ error: null }),
  reset: () => set({ pipeline: null, loading: false, error: null }),
}));
