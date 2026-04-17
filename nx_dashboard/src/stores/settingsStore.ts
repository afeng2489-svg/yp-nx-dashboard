import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface LayoutSettings {
  sidebarOpen: boolean;
  compactMode: boolean;
  animations: boolean;
}

export interface NotificationSettings {
  executionComplete: boolean;
  executionFailed: boolean;
  sessionUpdate: boolean;
  weeklyReport: boolean;
}

export interface SecuritySettings {
  sandboxExecution: boolean;
  resourceLimits: boolean;
  codeReview: boolean;
}

interface SettingsStore {
  layout: LayoutSettings;
  notifications: NotificationSettings;
  security: SecuritySettings;

  setLayout: (patch: Partial<LayoutSettings>) => void;
  setNotifications: (patch: Partial<NotificationSettings>) => void;
  setSecurity: (patch: Partial<SecuritySettings>) => void;
  reset: () => void;
}

const DEFAULT_LAYOUT: LayoutSettings = {
  sidebarOpen: true,
  compactMode: false,
  animations: true,
};

const DEFAULT_NOTIFICATIONS: NotificationSettings = {
  executionComplete: true,
  executionFailed: true,
  sessionUpdate: false,
  weeklyReport: true,
};

const DEFAULT_SECURITY: SecuritySettings = {
  sandboxExecution: true,
  resourceLimits: true,
  codeReview: true,
};

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      layout: { ...DEFAULT_LAYOUT },
      notifications: { ...DEFAULT_NOTIFICATIONS },
      security: { ...DEFAULT_SECURITY },

      setLayout: (patch) =>
        set((state) => ({ layout: { ...state.layout, ...patch } })),

      setNotifications: (patch) =>
        set((state) => ({ notifications: { ...state.notifications, ...patch } })),

      setSecurity: (patch) =>
        set((state) => ({ security: { ...state.security, ...patch } })),

      reset: () =>
        set({
          layout: { ...DEFAULT_LAYOUT },
          notifications: { ...DEFAULT_NOTIFICATIONS },
          security: { ...DEFAULT_SECURITY },
        }),
    }),
    {
      name: 'nexus-settings',
    }
  )
);
