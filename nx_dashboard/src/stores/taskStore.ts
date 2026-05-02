import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

export type TaskStatus =
  | 'queued'
  | 'delayed'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'timed_out';

export type TaskPriority = 'low' | 'normal' | 'high' | 'critical';

export interface StageRequest {
  name: string;
  agents: string[];
  prompt_template?: string;
  parallel?: boolean;
}

export interface QueuedTask {
  id: string;
  name: string;
  status: TaskStatus;
  priority: TaskPriority;
  retry_count: number;
  created_at: string;
  started_at?: string;
  finished_at?: string;
  error?: string;
}

export interface QueueStats {
  queued: number;
  running: number;
  completed: number;
  failed: number;
  scheduled_jobs: number;
}

export interface CreateTaskRequest {
  name: string;
  description: string;
  stages?: StageRequest[];
  variables?: Record<string, unknown>;
  priority?: TaskPriority;
}

export interface TaskListResponse {
  tasks: QueuedTask[];
  stats: QueueStats;
}

interface TaskStore {
  tasks: QueuedTask[];
  stats: QueueStats | null;
  loading: boolean;
  error: string | null;

  fetchTasks: () => Promise<void>;
  fetchStats: () => Promise<void>;
  createTask: (request: CreateTaskRequest) => Promise<QueuedTask | null>;
  getTask: (id: string) => Promise<QueuedTask | null>;
  cancelTask: (id: string) => Promise<boolean>;
  clearError: () => void;
}

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
      const task: QueuedTask = await response.json();
      set((state) => ({
        tasks: [...state.tasks, task],
        loading: false,
      }));
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
      const task: QueuedTask = await response.json();
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
      set((state) => ({
        tasks: state.tasks.map((task) =>
          task.id === id ? { ...task, status: 'cancelled' as TaskStatus } : task,
        ),
      }));
      get().fetchStats();
      return true;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to cancel task' });
      return false;
    }
  },

  clearError: () => set({ error: null }),
}));

export const taskStatusLabels: Record<TaskStatus, string> = {
  queued: '排队中',
  delayed: '延迟中',
  running: '运行中',
  completed: '已完成',
  failed: '已失败',
  cancelled: '已取消',
  timed_out: '已超时',
};

export const taskPriorityLabels: Record<TaskPriority, string> = {
  low: '低',
  normal: '普通',
  high: '高',
  critical: '紧急',
};

export const taskStatusColors: Record<TaskStatus, string> = {
  queued: 'bg-yellow-500/10 text-yellow-600 border-yellow-500/20',
  delayed: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  running: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  completed: 'bg-green-500/10 text-green-600 border-green-500/20',
  failed: 'bg-red-500/10 text-red-600 border-red-500/20',
  cancelled: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
  timed_out: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
};

export const taskPriorityColors: Record<TaskPriority, string> = {
  low: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
  normal: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  high: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  critical: 'bg-red-500/10 text-red-600 border-red-500/20',
};
