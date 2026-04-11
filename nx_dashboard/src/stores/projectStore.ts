import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import { onWorkspaceChange } from './workspaceStore';

// Project interfaces
export interface Project {
  id: string;
  name: string;
  description: string;
  team_id: string;
  workspace_id?: string;
  workflow_id?: string;
  variables: Record<string, string>;
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
  created_at: string;
  updated_at: string;
}

export interface ProjectMessage {
  id: string;
  project_id: string;
  role_id?: string;
  role_name?: string;
  content: string;
  message_type: string;
  created_at: string;
}

export interface ExecuteProjectResponse {
  success: boolean;
  project_id: string;
  team_id: string;
  messages: ProjectMessage[];
  final_output: string;
  error?: string;
}

// API_BASE_URL is imported from constants

// 自定义错误类型
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

// 带 timeout 的 fetch
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 15000
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

interface ProjectStore {
  projects: Project[];
  currentProject: Project | null;
  executionResult: ExecuteProjectResponse | null;
  loading: boolean;
  executing: boolean;
  error: string | null;

  // Project actions
  fetchProjects: () => Promise<void>;
  getProject: (id: string) => Promise<Project | null>;
  createProject: (project: Omit<Project, 'id' | 'created_at' | 'updated_at' | 'status' | 'variables'>) => Promise<Project>;
  updateProject: (id: string, project: Partial<Project>) => Promise<void>;
  deleteProject: (id: string) => Promise<void>;
  setCurrentProject: (project: Project | null) => void;

  // Execution actions
  executeProject: (projectId: string, task: string, context?: Record<string, string>) => Promise<ExecuteProjectResponse>;

  clearError: () => void;
  clearExecutionResult: () => void;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  currentProject: null,
  executionResult: null,
  loading: false,
  executing: false,
  error: null,

  fetchProjects: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/projects`, {}, 15000);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch projects: ${response.status} ${response.statusText}`,
          response.status
        );
      }

      const projects: Project[] = await response.json();
      set({ projects, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch projects: ${message}`,
      });
    }
  },

  getProject: async (id) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/projects/${id}`, {}, 10000
      );

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(
          `Failed to fetch project: ${response.status}`,
          response.status
        );
      }

      const data = await response.json();
      // API returns { project, team_name, workflow_name } for single project
      return data.project || data;
    } catch (error) {
      console.error('Failed to get project:', error);
      return null;
    }
  },

  createProject: async (project) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/projects`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: project.name,
          description: project.description,
          team_id: project.team_id,
          workspace_id: project.workspace_id || null,
          workflow_id: project.workflow_id || null,
        }),
      });

      if (!response.ok) {
        throw new ApiError(
          `Failed to create project: ${response.status}`,
          response.status
        );
      }

      const newProject: Project = await response.json();
      set((state) => ({
        projects: [newProject, ...state.projects],
        loading: false,
      }));
      return newProject;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ loading: false, error: message });
      throw error;
    }
  },

  updateProject: async (id, updates) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/projects/${id}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(updates),
        }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to update project: ${response.status}`,
          response.status
        );
      }

      const updatedProject: Project = await response.json();
      set((state) => ({
        projects: state.projects.map((p) =>
          p.id === id ? updatedProject : p
        ),
        loading: false,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ loading: false, error: message });
      throw error;
    }
  },

  deleteProject: async (id) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/projects/${id}`,
        { method: 'DELETE' }
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to delete project: ${response.status}`,
          response.status
        );
      }

      set((state) => ({
        projects: state.projects.filter((p) => p.id !== id),
        loading: false,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ loading: false, error: message });
      throw error;
    }
  },

  setCurrentProject: (project) => {
    set({ currentProject: project });
  },

  executeProject: async (projectId, task, context = {}) => {
    set({ executing: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/projects/${projectId}/execute`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            project_id: projectId,
            task,
            context,
          }),
        },
        120000 // 2 min timeout for execution
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to execute project: ${response.status}`,
          response.status
        );
      }

      const result: ExecuteProjectResponse = await response.json();
      set({ executing: false, executionResult: result });

      // Refresh project to get updated status
      const updatedProject = await get().getProject(projectId);
      if (updatedProject) {
        set((state) => ({
          projects: state.projects.map((p) =>
            p.id === projectId ? updatedProject : p
          ),
          currentProject: updatedProject,
        }));
      }

      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ executing: false, error: message });
      throw error;
    }
  },

  clearError: () => {
    set({ error: null });
  },

  clearExecutionResult: () => {
    set({ executionResult: null });
  },
}));

// Listen for workspace changes
onWorkspaceChange(() => {
  useProjectStore.getState().fetchProjects();
});