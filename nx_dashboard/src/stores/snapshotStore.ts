import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import { workspaceScopePath } from '../lib/workspaceScope';

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

  fetchProgress: (workspaceId: string) => Promise<void>;
  fetchSnapshots: (workspaceId: string) => Promise<void>;
  fetchHistory: (workspaceId: string, roleId: string) => Promise<void>;
  snapshotAll: (workspaceId: string) => Promise<void>;
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

  fetchProgress: async (workspaceId: string) => {
    set({ progressLoading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}${workspaceScopePath(workspaceId, 'progress')}`);
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

  fetchSnapshots: async (workspaceId: string) => {
    set({ snapshotsLoading: true, error: null });
    try {
      const res = await fetch(`${API_BASE_URL}${workspaceScopePath(workspaceId, 'role-snapshots')}`);
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

  fetchHistory: async (workspaceId: string, roleId: string) => {
    try {
      const res = await fetch(
        `${API_BASE_URL}${workspaceScopePath(workspaceId, `role-snapshots/${encodeURIComponent(roleId)}/history`)}`,
      );
      if (!res.ok) return;
      const data = await res.json();
      set({ history: { ...get().history, [roleId]: data } });
    } catch {
      // silent
    }
  },

  snapshotAll: async (workspaceId: string) => {
    try {
      await fetch(`${API_BASE_URL}${workspaceScopePath(workspaceId, 'snapshot-all')}`, {
        method: 'POST',
      });
    } catch {
      // silent
    }
  },

  clearError: () => set({ error: null }),
  reset: () =>
    set({
      progress: null,
      snapshots: [],
      history: {},
      progressLoading: false,
      snapshotsLoading: false,
      error: null,
    }),
}));
