import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

export type TaskType = 'workflow_execution' | 'code_review' | 'security_audit' | 'cleanup';

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface ScheduledTask {
  id: string;
  task_type: TaskType;
  payload: Record<string, unknown>;
  scheduled_at: string;
  execute_at: string;
  retry_count: number;
  max_retries: number;
  status: TaskStatus;
  error_message?: string;
  updated_at: string;
}

export interface QueueStats {
  pending: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
  total: number;
}

export interface CreateTaskRequest {
  task_type: TaskType;
  payload: Record<string, unknown>;
  delay_seconds?: number;
  max_retries?: number;
}

export interface TaskListResponse {
  tasks: ScheduledTask[];
  stats: QueueStats;
}

interface TaskStore {
  tasks: ScheduledTask[];
  stats: QueueStats | null;
  loading: boolean;
  error: string | null;

  fetchTasks: () => Promise<void>;
  fetchStats: () => Promise<void>;
  createTask: (request: CreateTaskRequest) => Promise<ScheduledTask | null>;
  getTask: (id: string) => Promise<ScheduledTask | null>;
  cancelTask: (id: string) => Promise<boolean>;
  clearError: () => void;
}

// API_BASE_URL is imported from constants

export const useTaskStore = create<TaskStore>((set, get) => ({
  tasks: [],
  stats: null,
  loading: false,
  error: null,

  fetchTasks: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/tasks`);
      if (!response.ok) {
        throw new Error(`API Error: ${response.status}`);
      }
      const data: TaskListResponse = await response.json();
      set({ tasks: data.tasks, stats: data.stats, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch tasks',
        loading: false,
      });
    }
  },

  fetchStats: async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/tasks/stats`);
      if (!response.ok) {
        throw new Error(`API Error: ${response.status}`);
      }
      const stats: QueueStats = await response.json();
      set({ stats });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to fetch stats' });
    }
  },

  createTask: async (request: CreateTaskRequest) => {
    set({ loading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/tasks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        throw new Error(`API Error: ${response.status}`);
      }
      const task: ScheduledTask = await response.json();
      set((state) => ({
        tasks: [...state.tasks, task],
        loading: false,
      }));
      // Refresh stats
      get().fetchStats();
      return task;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to create task',
        loading: false,
      });
      return null;
    }
  },

  getTask: async (id: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/tasks/${id}`);
      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new Error(`API Error: ${response.status}`);
      }
      const task: ScheduledTask = await response.json();
      return task;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to get task' });
      return null;
    }
  },

  cancelTask: async (id: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/tasks/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        if (response.status === 404) {
          return false;
        }
        throw new Error(`API Error: ${response.status}`);
      }
      // Update local state
      set((state) => ({
        tasks: state.tasks.map((task) =>
          task.id === id ? { ...task, status: 'cancelled' as TaskStatus } : task,
        ),
      }));
      // Refresh stats
      get().fetchStats();
      return true;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to cancel task' });
      return false;
    }
  },

  clearError: () => set({ error: null }),
}));

// Helper functions for task type and status display
export const taskTypeLabels: Record<TaskType, string> = {
  workflow_execution: '工作流执行',
  code_review: '代码审查',
  security_audit: '安全审计',
  cleanup: '清理任务',
};

export const taskStatusLabels: Record<TaskStatus, string> = {
  pending: '等待中',
  running: '运行中',
  completed: '已完成',
  failed: '已失败',
  cancelled: '已取消',
};

export const taskStatusColors: Record<TaskStatus, string> = {
  pending: 'bg-yellow-500/10 text-yellow-600 border-yellow-500/20',
  running: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  completed: 'bg-green-500/10 text-green-600 border-green-500/20',
  failed: 'bg-red-500/10 text-red-600 border-red-500/20',
  cancelled: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
};

export const taskTypeColors: Record<TaskType, string> = {
  workflow_execution: 'bg-indigo-500/10 text-indigo-600 border-indigo-500/20',
  code_review: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  security_audit: 'bg-red-500/10 text-red-600 border-red-500/20',
  cleanup: 'bg-green-500/10 text-green-600 border-green-500/20',
};
