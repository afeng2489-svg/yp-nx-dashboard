import { create } from 'zustand';

interface UIStore {
  sidebarOpen: boolean;
  activeTab: 'dashboard' | 'workflows' | 'executions' | 'sessions' | 'settings';
  terminalGrid: TerminalGridLayout;

  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setActiveTab: (tab: UIStore['activeTab']) => void;
  setTerminalGrid: (layout: Partial<TerminalGridLayout>) => void;
}

export interface TerminalGridLayout {
  columns: number;
  rows: number;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarOpen: true,
  activeTab: 'dashboard',
  terminalGrid: { columns: 2, rows: 2 },

  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),

  setSidebarOpen: (open) => set({ sidebarOpen: open }),

  setActiveTab: (tab) => set({ activeTab: tab }),

  setTerminalGrid: (layout) =>
    set((state) => ({
      terminalGrid: { ...state.terminalGrid, ...layout },
    })),
}));
