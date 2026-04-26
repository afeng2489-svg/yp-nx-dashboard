import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

// --- Types ---

export interface RoleSnapshot {
  id: string;
  project_id: string;
  team_id: string;
  role_id: string;
  role_name: string;
  phase: string;
  progress_pct: number;
  current_task: string;
  summary: string;
  last_cli_output: string;
  files_touched: string[];
  execution_count: number;
  checksum: string;
  created_at: string;
  updated_at: string;
}

export interface RoleSnapshotHistory {
  id: string;
  snapshot_id: string;
  project_id: string;
  role_id: string;
  phase: string;
  progress_pct: number;
  summary: string;
  created_at: string;
}

export interface ProjectProgress {
  project_id: string;
  team_id: string;
  pipeline_id?: string;
  overall_phase: string;
  overall_pct: number;
  total_roles: number;
  active_roles: number;
  completed_roles: number;
  failed_roles: number;
  last_activity: string;
  last_activity_at?: string;
  updated_at: string;
}

interface SnapshotState {
  progress: ProjectProgress | null;
  snapshots: RoleSnapshot[];
  history: Record<string, RoleSnapshotHistory[]>;
  progressLoading: boolean;
  snapshotsLoading: boolean;
  error: string | null;

  fetchProgress: (projectId: string) => Promise<void>;
  fetchSnapshots: (projectId: string) => Promise<void>;
  fetchHistory: (projectId: string, roleId: string) => Promise<void>;
  snapshotAll: (projectId: string) => Promise<void>;
  clearError: () => void;
  reset: () => void;
}

// --- Store ---

export const useSnapshotStore = create<SnapshotState>((set, get) => ({
  progress: null,
  snapshots: [],
  history: {},
  progressLoading: false,
  snapshotsLoading: false,
  error: null,

  fetchProgress: async (projectId: string) => {
    set({ progressLoading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/projects/${projectId}/progress`);
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ progress: data, progressLoading: false });
    } catch (err) {
      set({ error: (err as Error).message, progressLoading: false });
    }
  },

  fetchSnapshots: async (projectId: string) => {
    set({ snapshotsLoading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/projects/${projectId}/role-snapshots`);
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      set({ snapshots: data, snapshotsLoading: false });
    } catch (err) {
      set({ error: (err as Error).message, snapshotsLoading: false });
    }
  },

  fetchHistory: async (projectId: string, roleId: string) => {
    try {
      const res = await fetch(
        `${API_BASE_URL}/api/v1/projects/${projectId}/role-snapshots/${roleId}/history`
      );
      if (!res.ok) return;
      const data = await res.json();
      set({ history: { ...get().history, [roleId]: data } });
    } catch {
      // silent
    }
  },

  snapshotAll: async (projectId: string) => {
    try {
      await fetch(`${API_BASE_URL}/api/v1/projects/${projectId}/snapshot-all`, {
        method: 'POST',
      });
    } catch {
      // silent
    }
  },

  clearError: () => set({ error: null }),
  reset: () => set({ progress: null, snapshots: [], history: {}, progressLoading: false, snapshotsLoading: false, error: null }),
}));
