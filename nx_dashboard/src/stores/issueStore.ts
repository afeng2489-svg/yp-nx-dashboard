import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';

export type IssueStatus = 'discovered' | 'planned' | 'queued' | 'executing' | 'completed' | 'failed';
export type IssuePriority = 'critical' | 'high' | 'medium' | 'low';

export interface Issue {
  id: string;
  title: string;
  description: string;
  status: IssueStatus;
  priority: IssuePriority;
  perspectives: string[];
  solution: string | null;
  depends_on: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateIssueRequest {
  title: string;
  description: string;
  priority?: IssuePriority;
  perspectives?: string[];
  depends_on?: string[];
}

export interface UpdateIssueRequest {
  title?: string;
  description?: string;
  status?: IssueStatus;
  priority?: IssuePriority;
  solution?: string;
  perspectives?: string[];
  depends_on?: string[];
}

export const issueStatusLabels: Record<IssueStatus, string> = {
  discovered: '已发现',
  planned: '已规划',
  queued: '队列中',
  executing: '执行中',
  completed: '已完成',
  failed: '已失败',
};

export const issueStatusColors: Record<IssueStatus, string> = {
  discovered: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
  planned: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  queued: 'bg-yellow-500/10 text-yellow-600 border-yellow-500/20',
  executing: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  completed: 'bg-green-500/10 text-green-600 border-green-500/20',
  failed: 'bg-red-500/10 text-red-600 border-red-500/20',
};

export const issuePriorityLabels: Record<IssuePriority, string> = {
  critical: 'Critical',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
};

export const issuePriorityColors: Record<IssuePriority, string> = {
  critical: 'bg-red-500/10 text-red-700 border-red-500/30',
  high: 'bg-orange-500/10 text-orange-700 border-orange-500/30',
  medium: 'bg-yellow-500/10 text-yellow-700 border-yellow-500/30',
  low: 'bg-green-500/10 text-green-700 border-green-500/30',
};

interface IssueStore {
  issues: Issue[];
  loading: boolean;
  error: string | null;

  fetchIssues: (status?: IssueStatus, priority?: IssuePriority) => Promise<void>;
  createIssue: (req: CreateIssueRequest) => Promise<Issue | null>;
  updateIssue: (id: string, req: UpdateIssueRequest) => Promise<Issue | null>;
  deleteIssue: (id: string) => Promise<boolean>;
}

export const useIssueStore = create<IssueStore>((set) => ({
  issues: [],
  loading: false,
  error: null,

  fetchIssues: async (status?: IssueStatus, priority?: IssuePriority) => {
    set({ loading: true, error: null });
    try {
      const params = new URLSearchParams();
      if (status) params.set('status', status);
      if (priority) params.set('priority', priority);
      const url = `${API_BASE_URL}/api/v1/issues${params.toString() ? `?${params}` : ''}`;
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const issues: Issue[] = await res.json();
      set({ issues, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createIssue: async (req: CreateIssueRequest) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/issues`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(req),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const issue: Issue = await res.json();
      set((s) => ({ issues: [issue, ...s.issues] }));
      return issue;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateIssue: async (id: string, req: UpdateIssueRequest) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/issues/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(req),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const updated: Issue = await res.json();
      set((s) => ({ issues: s.issues.map((i) => (i.id === id ? updated : i)) }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteIssue: async (id: string) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/issues/${id}`, { method: 'DELETE' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      set((s) => ({ issues: s.issues.filter((i) => i.id !== id) }));
      return true;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },
}));
