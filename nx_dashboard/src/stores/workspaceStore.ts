import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Workspace {
  id: string;
  name: string;
  description?: string;
  root_path?: string;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface FileNode {
  id: string;
  name: string;
  path: string;
  is_directory: boolean;
  size: number;
  modified_at: string;
}

// Git diff types
export type GitDiffType = 'added' | 'modified' | 'deleted';

export interface GitDiff {
  path: string;
  filename: string;
  diff_type: GitDiffType;
  additions: number;
  deletions: number;
}

export interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  is_dirty: boolean;
}

// Event emitter for workspace changes with debouncing
type WorkspaceChangeListener = (workspace: Workspace | null) => void;
const listeners: Set<WorkspaceChangeListener> = new Set();
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let pendingWorkspace: Workspace | null = null;

export function onWorkspaceChange(listener: WorkspaceChangeListener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function notifyListeners(workspace: Workspace | null) {
  // Cancel any pending debounce
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }

  // Debounce workspace changes - wait 300ms before notifying
  // This prevents multiple rapid fetches when workspace changes
  pendingWorkspace = workspace;
  debounceTimer = setTimeout(() => {
    listeners.forEach(listener => listener(pendingWorkspace));
    debounceTimer = null;
  }, 300);
}

interface WorkspaceStore {
  workspaces: Workspace[];
  currentWorkspace: Workspace | null;
  files: FileNode[];
  currentPath: string;
  loading: boolean;
  filesLoading: boolean;
  error: string | null;

  fetchWorkspaces: () => Promise<void>;
  selectWorkspace: (workspace: Workspace | null) => void;
  createWorkspace: (name: string, description?: string, rootPath?: string) => Promise<Workspace | null>;
  updateWorkspace: (id: string, updates: Partial<Workspace>) => Promise<Workspace | null>;
  clearError: () => void;

  // File browsing
  browseFiles: (path?: string) => Promise<void>;
  navigateToPath: (path: string) => void;
  getParentPath: () => string;

  // Git operations
  gitDiffs: GitDiff[];
  gitStatus: GitStatus | null;
  diffsLoading: boolean;
  fetchGitDiffs: () => Promise<void>;
  getFileDiff: (filePath: string) => Promise<string>;
  fetchGitStatus: () => Promise<void>;
}

// Use relative path for Vite dev server proxy, or full URL for production
const API_BASE = import.meta.env.VITE_API_BASE_URL
  ? import.meta.env.VITE_API_BASE_URL
  : '';

// Custom error type
class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Fetch with timeout helper
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 8000
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

export const useWorkspaceStore = create<WorkspaceStore>()(
  persist(
    (set, get) => ({
      workspaces: [],
      currentWorkspace: null,
      files: [],
      currentPath: '',
      loading: false,
      filesLoading: false,
      error: null,
      gitDiffs: [],
      gitStatus: null,
      diffsLoading: false,

      fetchWorkspaces: async () => {
        set({ loading: true, error: null });
        try {
          const response = await fetchWithTimeout(`${API_BASE}/api/v1/workspaces`);
          if (!response.ok) {
            throw new ApiError(`Failed to fetch workspaces: ${response.status}`, response.status);
          }
          const workspaces: Workspace[] = await response.json();
          set({ workspaces, loading: false });

          // Auto-select first workspace if none selected and we have workspaces
          if (!get().currentWorkspace && workspaces.length > 0) {
            set({ currentWorkspace: workspaces[0] });
            notifyListeners(workspaces[0]);
            // Auto-browse files of selected workspace
            if (workspaces[0].root_path) {
              get().browseFiles();
            }
          }
        } catch (error) {
          const message = error instanceof Error ? error.message : 'Failed to fetch workspaces';
          set({
            loading: false,
            error: message,
          });
        }
      },

      selectWorkspace: (workspace) => {
        set({ currentWorkspace: workspace, currentPath: '', files: [] });
        notifyListeners(workspace);
        // Browse root directory when workspace changes
        if (workspace?.root_path) {
          get().browseFiles();
        }
      },

      createWorkspace: async (name, description, rootPath) => {
        set({ error: null });
        try {
          const response = await fetchWithTimeout(`${API_BASE}/api/v1/workspaces`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, description, root_path: rootPath }),
          });
          if (!response.ok) {
            throw new ApiError(`Failed to create workspace: ${response.status}`, response.status);
          }
          const workspace: Workspace = await response.json();
          set((state) => ({
            workspaces: [...state.workspaces, workspace],
            currentWorkspace: workspace,
          }));
          notifyListeners(workspace);
          // Browse files after creating workspace
          if (workspace.root_path) {
            get().browseFiles();
          }
          return workspace;
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to create workspace',
          });
          return null;
        }
      },

      updateWorkspace: async (id, updates) => {
        set({ error: null });
        try {
          const response = await fetchWithTimeout(`${API_BASE}/api/v1/workspaces/${id}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updates),
          });
          if (!response.ok) {
            throw new ApiError(`Failed to update workspace: ${response.status}`, response.status);
          }
          const workspace: Workspace = await response.json();
          set((state) => ({
            workspaces: state.workspaces.map(w => w.id === id ? workspace : w),
            currentWorkspace: state.currentWorkspace?.id === id ? workspace : state.currentWorkspace,
          }));
          // Re-browse if root_path changed
          if (updates.root_path && get().currentWorkspace?.id === id) {
            get().browseFiles();
          }
          return workspace;
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to update workspace',
          });
          return null;
        }
      },

      clearError: () => set({ error: null }),

      browseFiles: async (path) => {
        const workspace = get().currentWorkspace;
        if (!workspace?.id) {
          set({ files: [], currentPath: '' });
          return;
        }

        set({ filesLoading: true, error: null });
        try {
          const url = path
            ? `${API_BASE}/api/v1/workspaces/${workspace.id}/browse?path=${encodeURIComponent(path)}`
            : `${API_BASE}/api/v1/workspaces/${workspace.id}/browse`;
          const response = await fetchWithTimeout(url);
          if (!response.ok) {
            throw new ApiError(`Failed to browse files: ${response.status}`, response.status);
          }
          const files: FileNode[] = await response.json();
          set({
            files,
            currentPath: path || '',
            filesLoading: false,
          });
        } catch (error) {
          set({
            files: [],
            filesLoading: false,
            error: error instanceof Error ? error.message : 'Failed to browse files',
          });
        }
      },

      navigateToPath: (path) => {
        set({ currentPath: path });
        get().browseFiles(path);
      },

      getParentPath: () => {
        const path = get().currentPath;
        if (!path) return '';
        const parts = path.split('/');
        parts.pop();
        return parts.join('/');
      },

      // Git operations
      fetchGitDiffs: async () => {
        const workspace = get().currentWorkspace;
        if (!workspace?.id) {
          set({ gitDiffs: [] });
          return;
        }

        set({ diffsLoading: true });
        try {
          const response = await fetchWithTimeout(
            `${API_BASE}/api/v1/workspaces/${workspace.id}/diffs`,
            {},
            10000
          );
          if (!response.ok) {
            throw new ApiError(`Failed to fetch git diffs: ${response.status}`, response.status);
          }
          const diffs: GitDiff[] = await response.json();
          set({ gitDiffs: diffs, diffsLoading: false });
        } catch (error) {
          set({ gitDiffs: [], diffsLoading: false });
        }
      },

      getFileDiff: async (filePath: string) => {
        const workspace = get().currentWorkspace;
        if (!workspace?.id) {
          return '';
        }

        try {
          const response = await fetchWithTimeout(
            `${API_BASE}/api/v1/workspaces/${workspace.id}/diff/${encodeURIComponent(filePath)}`,
            {},
            10000
          );
          if (!response.ok) {
            return '';
          }
          const data = await response.json();
          return data.content || '';
        } catch {
          return '';
        }
      },

      fetchGitStatus: async () => {
        const workspace = get().currentWorkspace;
        if (!workspace?.id) {
          set({ gitStatus: null });
          return;
        }

        try {
          const response = await fetchWithTimeout(
            `${API_BASE}/api/v1/workspaces/${workspace.id}/git/status`,
            {},
            10000
          );
          if (!response.ok) {
            throw new ApiError(`Failed to fetch git status: ${response.status}`, response.status);
          }
          const status: GitStatus = await response.json();
          set({ gitStatus: status });
        } catch {
          set({ gitStatus: null });
        }
      },
    }),
    {
      name: 'nexus-workspace', // localStorage key
      partialize: (state) => ({ currentWorkspace: state.currentWorkspace }),
    }
  )
);