import { create } from 'zustand';
import { onWorkspaceChange } from './workspaceStore';

// Team interfaces
export interface Team {
  id: string;
  name: string;
  description?: string;
  created_at?: string;
  updated_at?: string;
  workspace_id?: string;
}

export interface Role {
  id: string;
  team_id?: string | null; // Optional: roles are global/shared in new architecture
  name: string;
  description?: string;
  instructions?: string;
  skills?: string[];
  model?: string;
  temperature?: number;
  created_at?: string;
  updated_at?: string;
}

export interface Message {
  id: string;
  team_id: string;
  role: 'user' | 'assistant' | 'system';
  role_id?: string | null;
  message_type: 'User' | 'Assistant' | 'System';
  content: string;
  created_at?: string;
}

export interface TelegramConfig {
  id: string;
  team_id: string;
  bot_token?: string;
  chat_id?: string;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface MemberBotStatus {
  role_id: string;
  role_name: string;
  bot_config: TelegramConfig | null;
  is_polling: boolean;
}

export interface MemberBotConfigItem {
  role_id: string;
  bot_token: string;
  chat_id?: string;
  notifications_enabled?: boolean;
  conversation_enabled?: boolean;
}

// Memory types
export interface MemorySearchResult {
  chunk_id: string;
  transcript_id: string;
  content: string;
  score: number;
  bm25_score: number;
  vector_score: number;
  created_at: string;
  metadata: Record<string, unknown>;
}

export interface MemorySearchResponse {
  results: MemorySearchResult[];
  total: number;
  query: string;
  search_time_ms: number;
}

export interface ExecutionResult {
  success: boolean;
  team_id: string;
  messages: Message[];
  final_output: string;
  error: string | null;
}

// API 配置 - 使用相对路径，Vite 开发服务器代理会处理 /api 请求
// 在 Tauri 生产模式，需要配置外部 URL 访问或使用 tauri-plugin-http
const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

// Helper to map backend role to frontend format
function mapBackendRole(r: any): Role {
  return {
    id: r.id,
    team_id: r.team_id,
    name: r.name,
    description: r.description,
    instructions: r.system_prompt,
    skills: r.skills || [],
    model: r.model_config?.model_id || r.model,
    temperature: r.model_config?.temperature ?? r.temperature,
    created_at: r.created_at,
    updated_at: r.updated_at,
  };
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

interface TeamStore {
  teams: Team[];
  currentTeam: Team | null;
  roles: Record<string, Role[]>; // Keyed by team_id
  messages: Message[];
  telegramConfig: TelegramConfig | null;
  loading: boolean;
  error: string | null;
  abortController: AbortController | null;

  // 监控模式：Record<teamId, enabled>，持久化到 localStorage
  teamMonitorMode: Record<string, boolean>;
  // 终端会话：Record<teamId, Record<roleId, sessionId>>，持久化到 localStorage
  terminalSessions: Record<string, Record<string, string>>;
  // 当前活跃的团队任务（监控模式下显示悬浮卡）
  activeTeamTask: {
    teamId: string;
    teamName: string;
    task: string;
    status: 'running' | 'done' | 'error' | 'waiting_confirmation';
    partialOutput?: string;
    result?: string;
    error?: string;
  } | null;

  // Team actions
  fetchTeams: () => Promise<void>;
  getTeam: (id: string) => Promise<Team | null>;
  createTeam: (team: Omit<Team, 'id'>) => Promise<Team>;
  updateTeam: (id: string, team: Partial<Team>) => Promise<void>;
  deleteTeam: (id: string) => Promise<void>;
  setCurrentTeam: (team: Team | null) => void;

  // Role actions
  fetchRoles: (teamId: string) => Promise<void>;
  createRole: (role: Omit<Role, 'id'>) => Promise<Role>;
  updateRole: (id: string, role: Partial<Role>) => Promise<void>;
  deleteRole: (id: string) => Promise<void>;
  listAllRoles: () => Promise<Role[]>;
  assignRoleToTeam: (roleId: string, teamId: string) => Promise<Role>;
  unassignRoleFromTeam: (roleId: string, teamId: string) => Promise<void>;

  // Skill actions
  assignSkill: (roleId: string, skillId: string) => Promise<void>;
  removeSkill: (roleId: string, skillId: string) => Promise<void>;

  // Message actions
  fetchMessages: (teamId: string) => Promise<void>;
  executeTask: (teamId: string, task: string) => Promise<ExecutionResult>;
  stopExecution: () => void;

  // Monitor mode actions
  setTeamMonitorMode: (teamId: string, enabled: boolean) => void;
  setActiveTeamTask: (task: TeamStore['activeTeamTask']) => void;
  clearActiveTeamTask: () => void;

  // Terminal session actions
  setTerminalSession: (teamId: string, roleId: string, sessionId: string | null) => void;

  // Telegram actions
  configureTelegram: (teamId: string, config: Partial<TelegramConfig>) => Promise<void>;
  getTelegramConfig: (teamId: string) => Promise<TelegramConfig | null>;
  enableTelegram: (teamId: string, enabled: boolean) => Promise<void>;
  // Per-member bot actions
  getMemberBots: (teamId: string) => Promise<MemberBotStatus[]>;
  configureMemberBot: (
    teamId: string,
    roleId: string,
    config: MemberBotConfigItem,
  ) => Promise<MemberBotStatus>;
  toggleAllMemberBots: (teamId: string, enabled: boolean) => Promise<MemberBotStatus[]>;

  // Memory actions
  searchMemory: (teamId: string, query: string) => Promise<MemorySearchResponse>;
  storeMemory: (
    teamId: string,
    userId: string,
    content: string,
    role?: string,
    sessionId?: string,
  ) => Promise<void>;
  clearMemory: (teamId: string) => Promise<void>;

  clearError: () => void;
}

// 从 localStorage 恢复监控模式设置
function loadMonitorMode(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem('team_monitor_mode');
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

// 从 localStorage 恢复终端会话（兼容旧的 Record<teamId, sessionId> 格式）
function loadTerminalSessions(): Record<string, Record<string, string>> {
  try {
    const raw = localStorage.getItem('team_terminal_sessions');
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    const result: Record<string, Record<string, string>> = {};
    for (const [teamId, val] of Object.entries(parsed)) {
      if (typeof val === 'string') {
        // 旧格式：直接是 sessionId — 废弃，跳过（session 已失效）
      } else if (typeof val === 'object' && val !== null) {
        result[teamId] = val as Record<string, string>;
      }
    }
    return result;
  } catch {
    return {};
  }
}

export const useTeamStore = create<TeamStore>((set, get) => ({
  teams: [],
  currentTeam: null,
  roles: {}, // Keyed by team_id
  messages: [],
  telegramConfig: null,
  loading: false,
  error: null,
  abortController: null,
  teamMonitorMode: loadMonitorMode(),
  terminalSessions: loadTerminalSessions(),
  activeTeamTask: null,

  // Team actions
  fetchTeams: async () => {
    set({ loading: true, error: null });
    try {
      const controller = new AbortController();
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/teams`,
        { signal: controller.signal },
        15000,
      );

      if (!response.ok) {
        throw new ApiError(
          `Failed to fetch teams: ${response.status} ${response.statusText}`,
          response.status,
        );
      }

      const teams: Team[] = await response.json();
      set({ teams, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({
        loading: false,
        error: `Failed to fetch teams: ${message}`,
      });
    }
  },

  getTeam: async (id) => {
    try {
      const response = await fetchWithTimeout(`${API_BASE}/api/v1/teams/${id}`, {}, 10000);

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to fetch team: ${response.status}`, response.status);
      }

      const data = await response.json();
      // API returns TeamWithRoles { team: Team, roles: [] }, extract just the team
      return data.team || data;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get team ${id}:`, message);
      return null;
    }
  },

  createTeam: async (team) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(team),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create team: ${response.status}`, response.status);
      }

      const newTeam = await response.json();
      set((state) => ({ teams: [...state.teams, newTeam] }));
      return newTeam;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create team: ${message}` });
      throw error;
    }
  },

  updateTeam: async (id, updates) => {
    // Optimistic update
    set((state) => ({
      teams: state.teams.map((t) =>
        t.id === id ? { ...t, ...updates, updated_at: new Date().toISOString() } : t,
      ),
    }));

    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to update team: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  deleteTeam: async (id) => {
    // Optimistic delete
    set((state) => ({
      teams: state.teams.filter((t) => t.id !== id),
      currentTeam: state.currentTeam?.id === id ? null : state.currentTeam,
    }));

    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to delete team: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  setCurrentTeam: (team) => set({ currentTeam: team }),

  // Role actions
  fetchRoles: async (teamId) => {
    try {
      const controller = new AbortController();
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/teams/${teamId}/roles`,
        { signal: controller.signal },
        10000,
      );

      if (!response.ok) {
        throw new ApiError(`Failed to fetch roles: ${response.status}`, response.status);
      }

      const backendRoles = await response.json();
      // Map backend response to frontend format
      const roles: Role[] = backendRoles.map(mapBackendRole);
      set((state) => ({ roles: { ...state.roles, [teamId]: roles } }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to fetch roles for team ${teamId}:`, message);
      set({ error: `Failed to fetch roles: ${message}` });
    }
  },

  createRole: async (role) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${role.team_id}/roles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(role),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to create role: ${response.status}`, response.status);
      }

      const backendRole = await response.json();
      const newRole = mapBackendRole(backendRole);
      // role.team_id is guaranteed to be set since we require it for creation
      const teamId = role.team_id!;
      set((state) => ({
        roles: {
          ...state.roles,
          [teamId]: [...(state.roles[teamId] || []), newRole],
        },
      }));
      return newRole;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to create role: ${message}` });
      throw error;
    }
  },

  updateRole: async (id, updates) => {
    const state = get();
    // Find the team_id for this role
    let teamId: string | undefined;
    for (const [tid, teamRoles] of Object.entries(state.roles)) {
      if (teamRoles.some((r) => r.id === id)) {
        teamId = tid;
        break;
      }
    }
    if (!teamId) return;

    // Optimistic update
    set((state) => ({
      roles: {
        ...state.roles,
        [teamId]: state.roles[teamId].map((r) =>
          r.id === id ? { ...r, ...updates, updated_at: new Date().toISOString() } : r,
        ),
      },
    }));

    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to update role: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  deleteRole: async (id) => {
    const state = get();
    // Find the team_id for this role
    let teamId: string | undefined;
    for (const [tid, teamRoles] of Object.entries(state.roles)) {
      if (teamRoles.some((r) => r.id === id)) {
        teamId = tid;
        break;
      }
    }
    if (!teamId) return;

    // Optimistic delete
    set((state) => ({
      roles: {
        ...state.roles,
        [teamId]: state.roles[teamId].filter((r) => r.id !== id),
      },
    }));

    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to delete role: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to sync with backend: ${message}` });
      throw error;
    }
  },

  // List all roles across all teams
  listAllRoles: async () => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles`);
      if (!response.ok) {
        throw new ApiError(`Failed to fetch all roles: ${response.status}`, response.status);
      }
      const backendRoles = await response.json();
      return backendRoles.map(mapBackendRole);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to fetch all roles: ${message}` });
      throw error;
    }
  },

  // Assign existing role to a team
  assignRoleToTeam: async (roleId, teamId) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${roleId}/team`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ team_id: teamId }),
      });
      if (!response.ok) {
        throw new ApiError(`Failed to assign role to team: ${response.status}`, response.status);
      }
      const backendRole = await response.json();
      const newRole = mapBackendRole(backendRole);
      // Add to the target team's roles (dedup by id to prevent duplicates)
      set((state) => {
        const existing = state.roles[teamId] || [];
        const already = existing.some((r) => r.id === newRole.id);
        return {
          roles: {
            ...state.roles,
            [teamId]: already ? existing : [...existing, newRole],
          },
        };
      });
      return newRole;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to assign role to team: ${message}` });
      throw error;
    }
  },

  // Remove role from team (only removes assignment, doesn't delete the role)
  unassignRoleFromTeam: async (roleId, teamId) => {
    set({ error: null });
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/roles/${roleId}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        throw new ApiError(`Failed to remove role from team: ${response.status}`, response.status);
      }
      // Remove from local state
      set((state) => ({
        roles: {
          ...state.roles,
          [teamId]: (state.roles[teamId] || []).filter((r) => r.id !== roleId),
        },
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to remove role from team: ${message}` });
      throw error;
    }
  },

  // Skill actions
  assignSkill: async (roleId, skillId) => {
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${roleId}/skills/${skillId}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to assign skill: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to assign skill: ${message}` });
      throw error;
    }
  },

  removeSkill: async (roleId, skillId) => {
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${roleId}/skills/${skillId}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to remove skill: ${response.status}`, response.status);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to remove skill: ${message}` });
      throw error;
    }
  },

  // Message actions
  fetchMessages: async (teamId) => {
    try {
      const controller = new AbortController();
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/teams/${teamId}/messages`,
        { signal: controller.signal },
        10000,
      );

      if (!response.ok) {
        throw new ApiError(`Failed to fetch messages: ${response.status}`, response.status);
      }

      const backendMessages: any[] = await response.json();
      // Map backend message_type to frontend role field
      const messages: Message[] = backendMessages.map((m) => ({
        ...m,
        role:
          m.message_type === 'User'
            ? 'user'
            : m.message_type === 'Assistant'
              ? 'assistant'
              : 'system',
      }));
      set({ messages });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to fetch messages for team ${teamId}:`, message);
      set({ error: `Failed to fetch messages: ${message}` });
    }
  },

  executeTask: async (teamId, task) => {
    set({ error: null });

    // Create new AbortController for this request
    const controller = new AbortController();
    set({ abortController: controller });

    // 如果该团队开启了监控模式，显示悬浮卡
    const { teamMonitorMode, teams } = get();
    const isMonitor = teamMonitorMode[teamId] ?? false;
    // auto_confirm: 监控模式 OFF = 自动确认，ON = 等待确认
    const autoConfirm = !isMonitor;
    if (isMonitor) {
      const team = teams.find((t) => t.id === teamId);
      set({
        activeTeamTask: {
          teamId,
          teamName: team?.name ?? '团队',
          task,
          status: 'running',
        },
      });
    }

    try {
      // Execute task - 记忆搜索和存储全部由后端处理
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/execute`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          team_id: teamId,
          task: task,
          context: {},
          auto_confirm: autoConfirm,
        }),
        signal: controller.signal,
      });

      if (!response.ok) {
        let errorMessage = `Failed to execute task: ${response.status}`;
        try {
          const errorBody = await response.json();
          if (errorBody?.error) {
            errorMessage = errorBody.error;
          } else if (errorBody?.message) {
            errorMessage = errorBody.message;
          }
        } catch {
          // Use status text if available
          if (response.statusText) {
            errorMessage = response.statusText;
          }
        }
        if (isMonitor) {
          set((state) => ({
            activeTeamTask: state.activeTeamTask
              ? { ...state.activeTeamTask, status: 'error', error: errorMessage }
              : null,
          }));
        }
        throw new ApiError(errorMessage, response.status);
      }

      const result: ExecutionResult = await response.json();
      console.log('[ExecuteTask] Execution result success:', result.success);

      if (isMonitor) {
        set((state) => ({
          activeTeamTask: state.activeTeamTask
            ? {
                ...state.activeTeamTask,
                status: result.success ? 'done' : 'error',
                result: result.final_output,
                error: result.error ?? undefined,
              }
            : null,
        }));
      }

      return result;
    } catch (error) {
      // Don't show error if it was aborted
      if (error instanceof Error && error.name === 'AbortError') {
        if (isMonitor) {
          set((state) => ({
            activeTeamTask: state.activeTeamTask
              ? { ...state.activeTeamTask, status: 'error', error: '已取消' }
              : null,
          }));
        }
        throw new ApiError('Task cancelled', 0);
      }
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to execute task: ${message}` });
      throw error;
    } finally {
      set({ abortController: null });
    }
  },

  stopExecution: () => {
    const { abortController } = get();
    if (abortController) {
      abortController.abort();
      set({ abortController: null });
    }
  },

  setTeamMonitorMode: (teamId, enabled) => {
    set((state) => {
      const next = { ...state.teamMonitorMode, [teamId]: enabled };
      try {
        localStorage.setItem('team_monitor_mode', JSON.stringify(next));
      } catch {
        /* ignore */
      }
      return { teamMonitorMode: next };
    });
  },

  setActiveTeamTask: (task) => set({ activeTeamTask: task }),

  clearActiveTeamTask: () => set({ activeTeamTask: null }),

  setTerminalSession: (teamId, roleId, sessionId) => {
    set((state) => {
      const teamSessions = { ...(state.terminalSessions[teamId] ?? {}) };
      if (sessionId) {
        teamSessions[roleId] = sessionId;
      } else {
        delete teamSessions[roleId];
      }
      const next = { ...state.terminalSessions };
      if (Object.keys(teamSessions).length > 0) {
        next[teamId] = teamSessions;
      } else {
        delete next[teamId];
      }
      try {
        localStorage.setItem('team_terminal_sessions', JSON.stringify(next));
      } catch {
        /* ignore */
      }
      return { terminalSessions: next };
    });
  },

  // Telegram actions
  configureTelegram: async (teamId, config) => {
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/telegram`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config),
      });

      if (!response.ok) {
        throw new ApiError(`Failed to configure Telegram: ${response.status}`, response.status);
      }

      const updatedConfig: TelegramConfig = await response.json();
      set({ telegramConfig: updatedConfig });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to configure Telegram: ${message}` });
      throw error;
    }
  },

  getTelegramConfig: async (teamId) => {
    try {
      const response = await fetchWithTimeout(
        `${API_BASE}/api/v1/teams/${teamId}/telegram`,
        {},
        10000,
      );

      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new ApiError(`Failed to get Telegram config: ${response.status}`, response.status);
      }

      const config: TelegramConfig = await response.json();
      set({ telegramConfig: config });
      return config;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      console.error(`Failed to get Telegram config for team ${teamId}:`, message);
      return null;
    }
  },

  enableTelegram: async (teamId, enabled) => {
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/telegram/${enabled}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });

      if (!response.ok) {
        throw new ApiError(
          `Failed to update Telegram settings: ${response.status}`,
          response.status,
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      set({ error: `Failed to update Telegram settings: ${message}` });
      throw error;
    }
  },

  getMemberBots: async (teamId) => {
    const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/members/bots`);
    if (!response.ok) throw new Error(`Failed to get member bots: ${response.status}`);
    return response.json();
  },

  configureMemberBot: async (teamId, roleId, config) => {
    const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/members/${roleId}/bot`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
    if (!response.ok) throw new Error(`Failed to configure member bot: ${response.status}`);
    return response.json();
  },

  toggleAllMemberBots: async (teamId, enabled) => {
    const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/members/bots/${enabled}`, {
      method: 'POST',
    });
    if (!response.ok) throw new Error(`Failed to toggle member bots: ${response.status}`);
    return response.json();
  },

  // Memory actions
  searchMemory: async (teamId, query) => {
    console.log('[Memory] Searching memory for team:', teamId, 'query:', query);
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/memories/search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ team_id: teamId, query, top_k: 5 }),
      });

      if (!response.ok) {
        console.error('[Memory] Search failed with status:', response.status);
        throw new ApiError(`Failed to search memory: ${response.status}`, response.status);
      }

      const result = (await response.json()) as MemorySearchResponse;
      console.log('[Memory] Search results:', result.results.length, 'results');
      return result;
    } catch (error) {
      console.error(`[Memory] Failed to search memory for team ${teamId}:`, error);
      return { results: [], total: 0, query, search_time_ms: 0 };
    }
  },

  storeMemory: async (teamId, userId, content, role = 'user', sessionId) => {
    console.log(
      '[Memory] Storing memory for team:',
      teamId,
      'userId:',
      userId,
      'content:',
      content.substring(0, 50),
    );
    try {
      const url = `${API_BASE}/api/v1/teams/${teamId}/memories`;
      console.log('[Memory] POST URL:', url);
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          team_id: teamId,
          user_id: userId,
          role,
          content,
          session_id: sessionId,
        }),
      });

      console.log('[Memory] Store response status:', response.status);

      if (!response.ok) {
        const errorText = await response.text();
        console.error('[Memory] Store failed with status:', response.status, 'body:', errorText);
        throw new ApiError(`Failed to store memory: ${response.status}`, response.status);
      }

      const result = await response.json();
      console.log('[Memory] Store success:', result);
    } catch (error) {
      console.error(`[Memory] Failed to store memory for team ${teamId}:`, error);
    }
  },

  clearMemory: async (teamId) => {
    try {
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/memories`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new ApiError(`Failed to clear memory: ${response.status}`, response.status);
      }
    } catch (error) {
      console.error(`Failed to clear memory for team ${teamId}:`, error);
    }
  },

  clearError: () => set({ error: null }),
}));
