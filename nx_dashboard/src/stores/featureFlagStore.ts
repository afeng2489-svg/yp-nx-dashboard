import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { API_BASE_URL } from '../api/constants';

// --- Types ---

export type FlagState = 'on' | 'readonly' | 'off';

export interface FeatureFlag {
  key: string;
  state: FlagState;
  circuit_breaker: boolean;
  error_count: number;
  error_threshold: number;
}

interface FeatureFlagState {
  flags: FeatureFlag[];
  loading: boolean;
  error: string | null;

  fetchFlags: () => Promise<void>;
  updateFlag: (key: string, state: FlagState) => Promise<void>;
  resetFlag: (key: string) => Promise<void>;
  isEnabled: (key: string) => boolean;
  clearError: () => void;
}

// --- Store ---

export const useFeatureFlagStore = create<FeatureFlagState>()(
  persist(
    (set, get) => ({
      flags: [],
      loading: false,
      error: null,

      fetchFlags: async () => {
        set({ loading: true, error: null });
        try {
          const res = await fetch(`${API_BASE_URL}/api/v1/feature-flags`);
          if (!res.ok) {
            const body = await res.json().catch(() => ({}));
            throw new Error(body.error || `HTTP ${res.status}`);
          }
          const data = await res.json();
          set({ flags: data, loading: false });
        } catch (err) {
          set({ error: (err as Error).message, loading: false });
        }
      },

      updateFlag: async (key: string, state: FlagState) => {
        set({ error: null });
        try {
          const res = await fetch(`${API_BASE_URL}/api/v1/feature-flags/${key}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ state }),
          });
          if (!res.ok) {
            const body = await res.json().catch(() => ({}));
            throw new Error(body.error || `HTTP ${res.status}`);
          }
          const updated = await res.json();
          set({
            flags: get().flags.map(f => f.key === key ? updated : f),
          });
        } catch (err) {
          set({ error: (err as Error).message });
        }
      },

      resetFlag: async (key: string) => {
        set({ error: null });
        try {
          const res = await fetch(`${API_BASE_URL}/api/v1/feature-flags/${key}/reset`, {
            method: 'POST',
          });
          if (!res.ok) {
            const body = await res.json().catch(() => ({}));
            throw new Error(body.error || `HTTP ${res.status}`);
          }
          const updated = await res.json();
          set({
            flags: get().flags.map(f => f.key === key ? updated : f),
          });
        } catch (err) {
          set({ error: (err as Error).message });
        }
      },

      isEnabled: (key: string) => {
        const flag = get().flags.find(f => f.key === key);
        return flag ? flag.state === 'on' && !flag.circuit_breaker : false;
      },

      clearError: () => set({ error: null }),
    }),
    {
      name: 'nexusflow-feature-flags',
      partialize: (state) => ({ flags: state.flags }),
    }
  )
);
