import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { DEFAULT_LAYOUT_MODE, normalizeLayoutMode, type LayoutMode } from '@/data/layoutModes';
import { DEFAULT_LAYOUT_VARIANT, type LayoutVariant } from '@/data/layoutVariants';

export interface LayoutSettings {
  /** AF-11 布局：guided | studio（旧版 focus 已迁移为 guided） */
  mode: LayoutMode;
  /** AF-11 经典 vs 精炼视觉 */
  variant: LayoutVariant;
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

export type ApprovalPolicy = 'trust_gates_final_only' | 'approve_all';
export type TextLaneCostMode = 'cost' | 'quality';

export interface FactorySettings {
  /** AF-UX-08：审批策略 */
  approvalPolicy: ApprovalPolicy;
  /** 待批准桌面通知 */
  approvalDesktopNotify: boolean;
  /** AF-MM-04：文本车道成本偏好 */
  textLaneCostMode: TextLaneCostMode;
}

interface SettingsStore {
  layout: LayoutSettings;
  notifications: NotificationSettings;
  security: SecuritySettings;
  services: ServiceEntry[];
  factory: FactorySettings;

  setLayout: (patch: Partial<LayoutSettings>) => void;
  setNotifications: (patch: Partial<NotificationSettings>) => void;
  setSecurity: (patch: Partial<SecuritySettings>) => void;
  setFactory: (patch: Partial<FactorySettings>) => void;
  setServices: (services: ServiceEntry[]) => void;
  updateService: (id: string, patch: Partial<ServiceEntry>) => void;
  reset: () => void;
}

const DEFAULT_LAYOUT: LayoutSettings = {
  mode: DEFAULT_LAYOUT_MODE,
  variant: DEFAULT_LAYOUT_VARIANT,
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

const DEFAULT_FACTORY: FactorySettings = {
  approvalPolicy: 'approve_all',
  approvalDesktopNotify: true,
  textLaneCostMode: 'quality',
};

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      layout: { ...DEFAULT_LAYOUT },
      notifications: { ...DEFAULT_NOTIFICATIONS },
      security: { ...DEFAULT_SECURITY },
      services: [...DEFAULT_SERVICES],
      factory: { ...DEFAULT_FACTORY },

      setLayout: (patch) =>
        set((state) => {
          const next = { ...state.layout, ...patch };
          if (patch.mode !== undefined) {
            next.mode = normalizeLayoutMode(patch.mode);
          }
          return { layout: next };
        }),

      setNotifications: (patch) =>
        set((state) => ({ notifications: { ...state.notifications, ...patch } })),

      setSecurity: (patch) => set((state) => ({ security: { ...state.security, ...patch } })),

      setFactory: (patch) => set((state) => ({ factory: { ...state.factory, ...patch } })),

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
          factory: { ...DEFAULT_FACTORY },
        }),
    }),
    {
      name: 'nexus-settings',
      version: 3,
      migrate: (persisted, version) => {
        const state = persisted as {
          layout?: { mode?: string; variant?: string };
          factory?: Partial<FactorySettings>;
        };
        if (state?.layout?.mode === 'focus') {
          state.layout.mode = 'guided';
        }
        if (version < 2 && state?.layout) {
          state.layout.variant = 'refined';
        }
        if (version < 3) {
          state.factory = { ...DEFAULT_FACTORY, ...state?.factory };
        }
        return persisted;
      },
    },
  ),
);
