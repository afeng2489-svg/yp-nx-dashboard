import { unwrapEnvelope } from '../api/response';
import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import { unwrapEnvelope, fetchWithTimeout } from '../api/response';

export interface Stage {
  name: string;
  agents: string[];
  parallel: boolean;
}

export interface Agent {
  id: string;
  role: string;
  model: string;
  prompt: string;
  depends_on?: string[];
}

export interface Template {
  id: string;
  name: string;
  description: string;
  category: string;
  stages: Stage[];
  agents: Agent[];
  variables: Record<string, unknown>;
}

export interface TemplateSummary {
  id: string;
  name: string;
  description: string;
  category: string;
  stage_count: number;
  agent_count: number;
}

export interface InstantiateResponse {
  workflow_id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface CreateTemplateRequest {
  name: string;
  description: string;
  category: string;
  stages: Stage[];
  agents: Agent[];
}

export interface InstantiateRequest {
  variables?: Record<string, unknown>;
}

export type TemplateCategory =
  | 'planning'
  | 'development'
  | 'analysis'
  | 'security'
  | 'testing'
  | 'research'
  | 'writing';

export const TEMPLATE_CATEGORIES: { value: TemplateCategory; label: string }[] = [
  { value: 'planning', label: '规划' },
  { value: 'development', label: '开发' },
  { value: 'analysis', label: '分析' },
  { value: 'security', label: '安全' },
  { value: 'testing', label: '测试' },
  { value: 'research', label: '调研' },
  { value: 'writing', label: '写作' },
];

// API_BASE_URL is imported from constants

interface TemplateStore {
  templates: TemplateSummary[];
  currentTemplate: Template | null;
  loading: boolean;
  error: string | null;
  selectedCategory: TemplateCategory | null;

  fetchTemplates: () => Promise<void>;
  fetchTemplatesByCategory: (category: string) => Promise<void>;
  getTemplate: (id: string) => Promise<Template | null>;
  createTemplate: (template: CreateTemplateRequest) => Promise<Template>;
  instantiateTemplate: (
    id: string,
    variables?: Record<string, unknown>,
  ) => Promise<InstantiateResponse>;
  setCurrentTemplate: (template: Template | null) => void;
  setSelectedCategory: (category: TemplateCategory | null) => void;
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
  timeout = 10000,
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

export const useTemplateStore = create<TemplateStore>((set) => ({
  templates: [],
  currentTemplate: null,
  loading: false,
  error: null,
  selectedCategory: null,

  fetchTemplates: async () => {
    set({ loading: true, error: null });
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/templates`);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch templates: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const data = unwrapEnvelope(await response.json());
      set({ templates: data.items || [], loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch templates: ${message}`,
      });
    }
  },

  fetchTemplatesByCategory: async (category: string) => {
    set({ loading: true, error: null, selectedCategory: category as TemplateCategory });
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/templates/category/${category}`,
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch templates: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const data = unwrapEnvelope(await response.json());
      set({ templates: data.items || [], loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch templates: ${message}`,
      });
    }
  },

  getTemplate: async (id: string) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/templates/${id}`);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch template: ${response.status}`, response.status);
      }

      const template = unwrapEnvelope(await response.json());
      set({ currentTemplate: template });
      return template;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get template ${id}:`, message);
      return null;
    }
  },

  createTemplate: async (templateRequest: CreateTemplateRequest) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/templates`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(templateRequest),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create template: ${response.status}`, response.status);
      }

      const newTemplate = unwrapEnvelope(await response.json());
      set((state) => ({
        templates: [
          ...state.templates,
          {
            id: newTemplate.id,
            name: newTemplate.name,
            description: newTemplate.description,
            category: newTemplate.category,
            stage_count: newTemplate.stages.length,
            agent_count: newTemplate.agents.length,
          },
        ],
      }));
      return newTemplate;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create template: ${message}` });
      throw error;
    }
  },

  instantiateTemplate: async (id: string, variables?: Record<string, unknown>) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/templates/${id}/instantiate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ variables } as InstantiateRequest),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to instantiate template: ${response.status}`, response.status);
      }

      const result: InstantiateResponse = unwrapEnvelope(await response.json());
      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to instantiate template: ${message}` });
      throw error;
    }
  },

  setCurrentTemplate: (template) => set({ currentTemplate: template }),

  setSelectedCategory: (category) => set({ selectedCategory: category }),

  clearError: () => set({ error: null }),
}));
