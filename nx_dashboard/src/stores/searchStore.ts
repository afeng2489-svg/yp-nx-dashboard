import { create } from 'zustand';
import { API_BASE_URL } from '../api/constants';
import type { SearchResult, SearchMode, IndexResponse } from '@/types/search';

// API_BASE_URL is imported from constants

interface SearchState {
  results: SearchResult | null;
  isLoading: boolean;
  error: string | null;
  query: string;
  mode: SearchMode;
  lastSearchTime: number | null;
  isIndexing: boolean;
  indexInfo: IndexResponse | null;
}

interface SearchActions {
  setQuery: (query: string) => void;
  setMode: (mode: SearchMode) => void;
  search: (query: string, mode: SearchMode) => Promise<void>;
  reindex: (workspacePath: string) => Promise<IndexResponse | null>;
  clearResults: () => void;
}

type SearchStore = SearchState & SearchActions;

export const useSearchStore = create<SearchStore>((set, get) => ({
  // State
  results: null,
  isLoading: false,
  error: null,
  query: '',
  mode: 'hybrid',
  lastSearchTime: null,
  isIndexing: false,
  indexInfo: null,

  // Actions
  setQuery: (query) => set({ query }),

  setMode: (mode) => set({ mode }),

  search: async (query, mode) => {
    if (!query.trim()) {
      set({ results: null, error: null });
      return;
    }

    set({ isLoading: true, error: null, query, mode });

    try {
      const params = new URLSearchParams({
        q: query,
        mode: mode,
        limit: '20',
      });

      const response = await fetch(`${API_BASE_URL}/api/v1/search?${params}`);

      if (!response.ok) {
        throw new Error(`Search failed: ${response.statusText}`);
      }

      const data = await response.json();

      set({
        results: data,
        isLoading: false,
        lastSearchTime: Date.now(),
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Search failed',
        isLoading: false,
        results: null,
      });
    }
  },

  reindex: async (workspacePath: string) => {
    set({ isIndexing: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/search/index`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ workspace_path: workspacePath, force: true }),
      });

      if (!response.ok) {
        throw new Error(`Reindex failed: ${response.statusText}`);
      }

      const data: IndexResponse = await response.json();
      set({ isIndexing: false, indexInfo: data });
      return data;
    } catch (error) {
      set({
        isIndexing: false,
        error: error instanceof Error ? error.message : 'Reindex failed',
      });
      return null;
    }
  },

  clearResults: () => set({
    results: null,
    error: null,
    query: '',
    lastSearchTime: null,
  }),
}));
