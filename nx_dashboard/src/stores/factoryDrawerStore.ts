import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type FactoryDrawerTab = 'terminal' | 'editor' | 'browser';

const MIN_HEIGHT = 160;
const MAX_HEIGHT_RATIO = 0.75;
export const FACTORY_DRAWER_DEFAULT_HEIGHT = 320;

export function clampFactoryDrawerHeight(height: number): number {
  if (typeof window === 'undefined') return FACTORY_DRAWER_DEFAULT_HEIGHT;
  const max = Math.floor(window.innerHeight * MAX_HEIGHT_RATIO);
  return Math.min(Math.max(height, MIN_HEIGHT), max);
}

interface FactoryDrawerState {
  isOpen: boolean;
  activeTab: FactoryDrawerTab;
  /** 工作室 · 底部集成面板是否展开 */
  integratedVisible: boolean;
  /** 终端/内容区高度（不含顶栏） */
  contentHeight: number;
  /** 是否曾打开过（用于关闭后保持 PTY 会话，仅隐藏） */
  terminalEverOpened: boolean;
  /** 仅「新会话」时递增，强制重建 PTY */
  sessionEpoch: number;
  open: (tab?: FactoryDrawerTab) => void;
  close: () => void;
  setTab: (tab: FactoryDrawerTab) => void;
  setContentHeight: (height: number) => void;
  resetTerminalSession: () => void;
  toggleIntegrated: () => void;
  showIntegrated: () => void;
  hideIntegrated: () => void;
}

export const useFactoryDrawerStore = create<FactoryDrawerState>()(
  persist(
    (set) => ({
      isOpen: false,
      activeTab: 'terminal',
      integratedVisible: false,
      contentHeight: FACTORY_DRAWER_DEFAULT_HEIGHT,
      terminalEverOpened: false,
      sessionEpoch: 0,
      open: (tab) =>
        set({
          isOpen: true,
          activeTab: tab ?? 'terminal',
          terminalEverOpened: true,
        }),
      close: () => set({ isOpen: false }),
      setTab: (tab) => set({ activeTab: tab }),
      setContentHeight: (height) => set({ contentHeight: clampFactoryDrawerHeight(height) }),
      resetTerminalSession: () => set((s) => ({ sessionEpoch: s.sessionEpoch + 1 })),
      toggleIntegrated: () =>
        set((s) => ({
          integratedVisible: !s.integratedVisible,
          terminalEverOpened: true,
        })),
      showIntegrated: () => set({ integratedVisible: true, terminalEverOpened: true }),
      hideIntegrated: () => set({ integratedVisible: false }),
    }),
    {
      name: 'nexusflow-factory-drawer',
      partialize: (state) => ({ contentHeight: state.contentHeight }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.contentHeight = clampFactoryDrawerHeight(state.contentHeight);
        }
      },
    },
  ),
);
