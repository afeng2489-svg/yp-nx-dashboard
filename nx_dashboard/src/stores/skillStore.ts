import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface SkillParameter {
  name: string;
  description: string;
  param_type: string;
  required: boolean;
  default?: unknown | null;
}

export interface SkillSummary {
  id: string;
  name: string;
  description: string;
  category: string;
  version: string;
  tags: string[];
  parameter_count: number;
  is_preset: boolean;
}

export interface SkillDetail {
  id: string;
  name: string;
  description: string;
  category: string;
  version: string;
  author: string | null;
  tags: string[];
  parameters: SkillParameter[];
  code: string | null;
  is_preset: boolean;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateSkillRequest {
  id: string;
  name: string;
  description: string;
  category: string;
  version?: string;
  author?: string;
  tags?: string[];
  parameters?: SkillParameter[];
  code?: string;
}

export interface UpdateSkillRequest {
  name?: string;
  description?: string;
  category?: string;
  version?: string;
  author?: string;
  tags?: string[];
  parameters?: SkillParameter[];
  code?: string;
  enabled?: boolean;
}

export interface SkillStats {
  total_skills: number;
  by_category: { category: string; count: number }[];
  by_tag: { tag: string; count: number }[];
}

export interface ExecuteSkillRequest {
  skill_id: string;
  phase?: string;
  params: Record<string, unknown>;
  working_dir?: string;
}

export interface ExecuteSkillResponse {
  success: boolean;
  skill_id: string;
  phase: string | null;
  output: unknown;
  error: string | null;
  duration_ms: number;
}

// API 配置
const API_BASE = import.meta.env.VITE_API_BASE_URL
  ? import.meta.env.VITE_API_BASE_URL
  : '';

// 带 timeout 的 fetch
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 10000
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
      throw new Error('Request timeout');
    }
    throw error;
  }
}

interface SkillStore {
  skills: SkillSummary[];
  currentSkill: SkillDetail | null;
  stats: SkillStats | null;
  categories: string[];
  tags: string[];
  searchResults: SkillSummary[];
  loading: boolean;
  saving: boolean;
  executing: boolean;
  error: string | null;

  // Actions - CRUD
  fetchSkills: () => Promise<void>;
  fetchSkill: (id: string) => Promise<SkillDetail | null>;
  createSkill: (skill: CreateSkillRequest) => Promise<SkillDetail | null>;
  updateSkill: (id: string, skill: UpdateSkillRequest) => Promise<SkillDetail | null>;
  deleteSkill: (id: string) => Promise<boolean>;
  fetchStats: () => Promise<void>;
  fetchCategories: () => Promise<void>;
  fetchTags: () => Promise<void>;
  searchSkills: (query: string) => Promise<void>;
  fetchByCategory: (category: string) => Promise<void>;
  fetchByTag: (tag: string) => Promise<void>;
  executeSkill: (request: ExecuteSkillRequest) => Promise<ExecuteSkillResponse>;
  clearSearch: () => void;
  clearError: () => void;
  clearCurrentSkill: () => void;
}

export const useSkillStore = create<SkillStore>((set, get) => ({
  skills: [],
  currentSkill: null,
  stats: null,
  categories: [],
  tags: [],
  searchResults: [],
  loading: false,
  saving: false,
  executing: false,
  error: null,

  fetchSkills: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills`);
      if (!response.ok) {
        throw new Error(`Failed to fetch skills: ${response.status}`);
      }
      const data: SkillSummary[] = await response.json();
      set({ skills: data, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch skills',
      });
    }
  },

  fetchSkill: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/${id}`);
      if (!response.ok) {
        if (response.status === 404) {
          set({ loading: false, currentSkill: null });
          return null;
        }
        throw new Error(`Failed to fetch skill: ${response.status}`);
      }
      const data: SkillDetail = await response.json();
      set({ loading: false, currentSkill: data });
      return data;
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch skill',
      });
      return null;
    }
  },

  createSkill: async (skill: CreateSkillRequest) => {
    set({ saving: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(skill),
      });
      if (!response.ok) {
        const err = await response.text();
        throw new Error(err || `Failed to create skill: ${response.status}`);
      }
      const data: SkillDetail = await response.json();
      set((state) => ({
        skills: [...state.skills, {
          id: data.id,
          name: data.name,
          description: data.description,
          category: data.category,
          version: data.version,
          tags: data.tags,
          parameter_count: data.parameters.length,
          is_preset: data.is_preset,
        }],
        saving: false,
      }));
      return data;
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : 'Failed to create skill',
      });
      return null;
    }
  },

  updateSkill: async (id: string, skill: UpdateSkillRequest) => {
    set({ saving: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(skill),
      });
      if (!response.ok) {
        const err = await response.text();
        throw new Error(err || `Failed to update skill: ${response.status}`);
      }
      const data: SkillDetail = await response.json();
      set((state) => ({
        skills: state.skills.map((s) =>
          s.id === id
            ? {
                ...s,
                name: data.name,
                description: data.description,
                category: data.category,
                version: data.version,
                tags: data.tags,
                parameter_count: data.parameters.length,
              }
            : s
        ),
        currentSkill: state.currentSkill?.id === id ? data : state.currentSkill,
        saving: false,
      }));
      return data;
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : 'Failed to update skill',
      });
      return null;
    }
  },

  deleteSkill: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        const err = await response.text();
        throw new Error(err || `Failed to delete skill: ${response.status}`);
      }
      set((state) => ({
        skills: state.skills.filter((s) => s.id !== id),
        currentSkill: state.currentSkill?.id === id ? null : state.currentSkill,
        loading: false,
      }));
      return true;
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to delete skill',
      });
      return false;
    }
  },

  fetchStats: async () => {
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/stats`);
      if (!response.ok) {
        throw new Error(`Failed to fetch stats: ${response.status}`);
      }
      const data: SkillStats = await response.json();
      set({ stats: data });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to fetch stats' });
    }
  },

  fetchCategories: async () => {
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/categories`);
      if (!response.ok) {
        throw new Error(`Failed to fetch categories: ${response.status}`);
      }
      const data: string[] = await response.json();
      set({ categories: data });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to fetch categories' });
    }
  },

  fetchTags: async () => {
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/skills/tags`);
      if (!response.ok) {
        throw new Error(`Failed to fetch tags: ${response.status}`);
      }
      const data: string[] = await response.json();
      set({ tags: data });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : 'Failed to fetch tags' });
    }
  },

  searchSkills: async (query: string) => {
    if (!query.trim()) {
      set({ searchResults: [] });
      return;
    }
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/skills/search?query=${encodeURIComponent(query)}`
      );
      if (!response.ok) {
        throw new Error(`Failed to search skills: ${response.status}`);
      }
      const data: SkillSummary[] = await response.json();
      set({ searchResults: data, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to search skills',
      });
    }
  },

  fetchByCategory: async (category: string) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/skills/category/${encodeURIComponent(category)}`
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch skills by category: ${response.status}`);
      }
      const data: SkillSummary[] = await response.json();
      set({ skills: data, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch skills by category',
      });
    }
  },

  fetchByTag: async (tag: string) => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/skills/tag/${encodeURIComponent(tag)}`
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch skills by tag: ${response.status}`);
      }
      const data: SkillSummary[] = await response.json();
      set({ skills: data, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch skills by tag',
      });
    }
  },

  executeSkill: async (request: ExecuteSkillRequest) => {
    set({ executing: true, error: null });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/skills/${request.skill_id}/execute`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(request),
        }
      );
      if (!response.ok) {
        throw new Error(`Failed to execute skill: ${response.status}`);
      }
      const data: ExecuteSkillResponse = await response.json();
      set({ executing: false });
      return data;
    } catch (error) {
      set({
        executing: false,
        error: error instanceof Error ? error.message : 'Failed to execute skill',
      });
      throw error;
    }
  },

  clearSearch: () => set({ searchResults: [] }),

  clearError: () => set({ error: null }),

  clearCurrentSkill: () => set({ currentSkill: null }),
}));
