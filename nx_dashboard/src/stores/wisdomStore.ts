import { unwrapEnvelope } from '../api/response';
import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import { unwrapEnvelope, fetchWithTimeout } from '../api/response';

export type WisdomCategory = 'learning' | 'decision' | 'convention' | 'pattern' | 'fix';

export interface WisdomEntry {
  id: string;
  category: WisdomCategory;
  title: string;
  content: string;
  tags: string[];
  source_session: string;
  confidence: number;
  created_at: string;
}

export interface CategorySummary {
  category: WisdomCategory;
  count: number;
}

export interface CreateWisdomRequest {
  category: WisdomCategory;
  title: string;
  content: string;
  tags?: string[];
  source_session: string;
  confidence?: number;
}

export interface QueryWisdomRequest {
  category?: WisdomCategory;
  tags?: string[];
  query?: string;
  min_confidence?: number;
  limit?: number;
  offset?: number;
}

export interface WisdomResponse {
  entries: WisdomEntry[];
  total: number;
  offset: number;
  limit: number;
}

// API_BASE_URL is imported from constants

interface WisdomStore {
  entries: WisdomEntry[];
  currentEntry: WisdomEntry | null;
  categories: CategorySummary[];
  loading: boolean;
  error: string | null;
  searchQuery: string;
  selectedCategory: WisdomCategory | null;

  // Actions
  fetchEntries: (params?: QueryWisdomRequest) => Promise<void>;
  fetchCategories: () => Promise<void>;
  getEntry: (id: string) => Promise<WisdomEntry | null>;
  createEntry: (request: CreateWisdomRequest) => Promise<WisdomEntry>;
  deleteEntry: (id: string) => Promise<void>;
  search: (query: string, limit?: number) => Promise<WisdomEntry[]>;
  setCurrentEntry: (entry: WisdomEntry | null) => void;
  setSearchQuery: (query: string) => void;
  setSelectedCategory: (category: WisdomCategory | null) => void;
  clearError: () => void;
}

// Custom error type
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

// Fetch with timeout helper
async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 5000,
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

// Get error message safely
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return 'Unknown error';
}

export const useWisdomStore = create<WisdomStore>((set, get) => ({
  entries: [],
  currentEntry: null,
  categories: [],
  loading: false,
  error: null,
  searchQuery: '',
  selectedCategory: null,

  fetchEntries: async (params?: QueryWisdomRequest) => {
    set({ loading: true, error: null });
    try {
      const searchParams = new URLSearchParams();
      if (params?.category) searchParams.set('category', params.category);
      if (params?.tags?.length) searchParams.set('tags', params.tags.join(','));
      if (params?.query) searchParams.set('query', params.query);
      if (params?.min_confidence) searchParams.set('min_confidence', String(params.min_confidence));
      if (params?.limit) searchParams.set('limit', String(params.limit));
      if (params?.offset) searchParams.set('offset', String(params.offset));

      const url = `${API_BASE_URL}/api/v1/wisdom${searchParams.toString() ? `?${searchParams}` : ''}`;
      const response = await fetchWithTimeout(url);

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch wisdom entries: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const data: WisdomResponse = unwrapEnvelope(await response.json());
      set({ entries: data.entries, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: `Failed to fetch wisdom: ${getErrorMessage(error)}`,
      });
    }
  },

  fetchCategories: async () => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/wisdom/categories`);

      if (!response.ok) {
        throw new ApiError(`Failed to fetch categories: ${response.status}`, response.status);
      }

      const data: CategorySummary[] = unwrapEnvelope(await response.json());
      set({ categories: data });
    } catch (error) {
      console.error('Failed to fetch categories:', getErrorMessage(error));
    }
  },

  getEntry: async (id: string) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE_URL}/api/v1/wisdom/${id}`);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch entry: ${response.status}`, response.status);
      }

      const data: WisdomEntry = unwrapEnvelope(await response.json());
      return data;
    } catch (error) {
      console.error(`Failed to get wisdom entry ${id}:`, getErrorMessage(error));
      return null;
    }
  },

  createEntry: async (request: CreateWisdomRequest) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/wisdom`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...request,
          tags: request.tags || [],
          confidence: request.confidence ?? 0.8,
        }),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create wisdom entry: ${response.status}`, response.status);
      }

      const newEntry: WisdomEntry = unwrapEnvelope(await response.json());
      set((state) => ({ entries: [newEntry, ...state.entries] }));
      return newEntry;
    } catch (error) {
      const message = getErrorMessage(error);
      set({ error: `Failed to create entry: ${message}` });
      throw error;
    }
  },

  deleteEntry: async (id: string) => {
    // Optimistic delete
    const previousEntries = get().entries;
    set((state) => ({
      entries: state.entries.filter((e) => e.id !== id),
      currentEntry: state.currentEntry?.id === id ? null : state.currentEntry,
    }));

    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/wisdom/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        // Revert optimistic update
        set({ entries: previousEntries });
        throw new ApiError(`Failed to delete entry: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = getErrorMessage(error);
      set({ error: `Failed to delete entry: ${message}` });
      throw error;
    }
  },

  search: async (query: string, limit = 20) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE_URL}/api/v1/wisdom/search?q=${encodeURIComponent(query)}&limit=${limit}`,
      );

      if (!response.ok) {
        throw new ApiError(`Failed to search: ${response.status}`, response.status);
      }

      const data = unwrapEnvelope(await response.json());
      return data.entries as WisdomEntry[];
    } catch (error) {
      console.error('Search failed:', getErrorMessage(error));
      return [];
    }
  },

  setCurrentEntry: (entry) => set({ currentEntry: entry }),

  setSearchQuery: (query) => set({ searchQuery: query }),

  setSelectedCategory: (category) => set({ selectedCategory: category }),

  clearError: () => set({ error: null }),
}));

// Utility function to get category display name
export function getCategoryDisplayName(category: WisdomCategory): string {
  const names: Record<WisdomCategory, string> = {
    learning: '学习',
    decision: '决策',
    convention: '规范',
    pattern: '模式',
    fix: '缺陷修复',
  };
  return names[category] || category;
}

// Utility function to get category color
export function getCategoryColor(category: WisdomCategory): string {
  const colors: Record<WisdomCategory, string> = {
    learning: 'blue',
    decision: 'purple',
    convention: 'green',
    pattern: 'orange',
    fix: 'red',
  };
  return colors[category] || 'gray';
}
