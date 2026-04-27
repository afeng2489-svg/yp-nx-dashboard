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

export interface ServiceEntry {
  id: string;
  name: string;
  command: string;
  cwd: string;
}

const DEFAULT_SERVICES: ServiceEntry[] = [
  {
    id: 'frontend',
    name: '前端',
    command: 'npm run dev',
    cwd: '', // user fills in or auto-detected
  },
  {
    id: 'backend',
    name: '后端',
    command: 'cargo run -p nx_api',
    cwd: '',
  },
];

interface SettingsStore {
  layout: LayoutSettings;
  notifications: NotificationSettings;
  security: SecuritySettings;
  services: ServiceEntry[];

  setLayout: (patch: Partial<LayoutSettings>) => void;
  setNotifications: (patch: Partial<NotificationSettings>) => void;
  setSecurity: (patch: Partial<SecuritySettings>) => void;
  setServices: (services: ServiceEntry[]) => void;
  updateService: (id: string, patch: Partial<ServiceEntry>) => void;
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
      services: [...DEFAULT_SERVICES],

      setLayout: (patch) => set((state) => ({ layout: { ...state.layout, ...patch } })),

      setNotifications: (patch) =>
        set((state) => ({ notifications: { ...state.notifications, ...patch } })),

      setSecurity: (patch) => set((state) => ({ security: { ...state.security, ...patch } })),

      setServices: (services) => set({ services }),

      updateService: (id, patch) =>
        set((state) => ({
          services: state.services.map((s) => (s.id === id ? { ...s, ...patch } : s)),
        })),

      reset: () =>
        set({
          layout: { ...DEFAULT_LAYOUT },
          notifications: { ...DEFAULT_NOTIFICATIONS },
          security: { ...DEFAULT_SECURITY },
          services: [...DEFAULT_SERVICES],
        }),
    }),
    {
      name: 'nexus-settings',
    },
  ),
);
