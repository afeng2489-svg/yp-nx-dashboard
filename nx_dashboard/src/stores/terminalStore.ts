import { create } from 'zustand';

export interface TerminalTab {
  id: string;
  title: string;
  sessionId?: string;
}

export interface TerminalInstance {
  id: string;
  tabId: string;
  title: string;
}

type GridLayout = '1x1' | '2x1' | '2x2' | '3x3';

// 窗口间同步的 Channel 名称
const SYNC_CHANNEL_NAME = 'nexusflow-terminal-sync';

// 同步消息类型
export interface SyncMessage {
  type:
    | 'tab_updated'
    | 'active_tab_changed'
    | 'layout_changed'
    | 'terminal_output'
    | 'sync_request'
    | 'sync_response'
    | 'fullscreen_changed'
    | 'ping'
    | 'pong';
  windowId: string;
  payload?: unknown;
}

// 完整的终端状态（用于同步）
export interface TerminalState {
  tabs: TerminalTab[];
  activeTabId: string | null;
  terminals: TerminalInstance[];
  gridLayout: GridLayout;
  isFullscreen: boolean;
}

interface TerminalStore {
  tabs: TerminalTab[];
  activeTabId: string | null;
  terminals: TerminalInstance[];
  gridLayout: GridLayout;
  isFullscreen: boolean;
  logs: Map<string, string[]>;

  addTab: (tab: Omit<TerminalTab, 'id'>) => string;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabTitle: (id: string, title: string) => void;

  addTerminal: (terminal: Omit<TerminalInstance, 'id'>) => string;
  removeTerminal: (id: string) => void;

  setGridLayout: (layout: GridLayout) => void;
  setFullscreen: (isFullscreen: boolean) => void;

  appendLog: (terminalId: string, log: string) => void;
  receiveLog: (terminalId: string, log: string) => void;
  clearLogs: (terminalId: string) => void;
  syncState: (state: Partial<TerminalState>) => void;
}

const generateId = () => Math.random().toString(36).substring(2, 9);

// 同步状态到其他窗口
let syncChannel: BroadcastChannel | null = null;
let syncWindowId: string | null = null;
let onSyncStateChange: ((state: Partial<TerminalState>) => void) | null = null;
let onSyncOutputReceive: ((terminalId: string, output: string) => void) | null = null;

// 初始化窗口同步
export function initWindowSync(
  windowId: string,
  onStateChange: (state: Partial<TerminalState>) => void,
  onOutputReceive: (terminalId: string, output: string) => void,
) {
  syncWindowId = windowId;
  onSyncStateChange = onStateChange;
  onSyncOutputReceive = onOutputReceive;

  if (!syncChannel) {
    syncChannel = new BroadcastChannel(SYNC_CHANNEL_NAME);

    syncChannel.onmessage = (event: MessageEvent<SyncMessage>) => {
      const msg = event.data;
      if (msg.windowId === syncWindowId) return;

      switch (msg.type) {
        case 'sync_request':
          // 不处理自己发的
          break;
        case 'ping':
          // 回复 pong
          syncChannel?.postMessage({ type: 'pong', windowId: syncWindowId! } as SyncMessage);
          break;
        case 'pong':
          // 不处理
          break;
        case 'sync_response':
          if (msg.payload && onSyncStateChange) {
            onSyncStateChange(msg.payload as Partial<TerminalState>);
          }
          break;
        case 'tab_updated':
        case 'active_tab_changed':
        case 'layout_changed':
        case 'fullscreen_changed':
          if (msg.payload && onSyncStateChange) {
            onSyncStateChange(msg.payload as Partial<TerminalState>);
          }
          break;
        case 'terminal_output':
          if (msg.payload && onSyncOutputReceive) {
            const { terminalId, output } = msg.payload as { terminalId: string; output: string };
            onSyncOutputReceive(terminalId, output);
          }
          break;
      }
    };
  }
}

// 广播状态变化
export function broadcastStateChange(type: SyncMessage['type'], payload: unknown) {
  if (syncChannel && syncWindowId) {
    syncChannel.postMessage({ type, windowId: syncWindowId, payload } as SyncMessage);
  }
}

// 广播终端输出
export function broadcastOutput(terminalId: string, output: string) {
  if (syncChannel && syncWindowId) {
    syncChannel.postMessage({
      type: 'terminal_output',
      windowId: syncWindowId,
      payload: { terminalId, output },
    } as SyncMessage);
  }
}

// 请求同步
export function requestSync() {
  if (syncChannel && syncWindowId) {
    syncChannel.postMessage({ type: 'sync_request', windowId: syncWindowId } as SyncMessage);
  }
}

// 销毁同步
export function destroyWindowSync() {
  if (syncChannel) {
    syncChannel.close();
    syncChannel = null;
  }
  syncWindowId = null;
  onSyncStateChange = null;
  onSyncOutputReceive = null;
}

// 导出当前状态
export function getCurrentTerminalState(
  tabs: TerminalTab[],
  activeTabId: string | null,
  terminals: TerminalInstance[],
  gridLayout: GridLayout,
  isFullscreen: boolean,
): TerminalState {
  return { tabs, activeTabId, terminals, gridLayout, isFullscreen };
}

export const useTerminalStore = create<TerminalStore>((set) => ({
  tabs: [{ id: 'default', title: '终端 1' }],
  activeTabId: 'default',
  terminals: [{ id: 'term-1', tabId: 'default', title: '终端 1' }],
  gridLayout: '2x2',
  isFullscreen: false,
  logs: new Map(),

  addTab: (tab) => {
    const id = generateId();
    const newTab: TerminalTab = { ...tab, id };
    set((state) => {
      const newState = { tabs: [...state.tabs, newTab] };
      // 广播变化
      broadcastStateChange('tab_updated', { tabs: newState.tabs });
      return newState;
    });
    return id;
  },

  removeTab: (id) => {
    set((state) => {
      const newTabs = state.tabs.filter((t) => t.id !== id);
      const newTerminals = state.terminals.filter((t) => t.tabId !== id);
      const newActiveTabId = state.activeTabId === id ? newTabs[0]?.id || null : state.activeTabId;
      const newState = {
        tabs: newTabs,
        terminals: newTerminals,
        activeTabId: newActiveTabId,
      };
      broadcastStateChange('tab_updated', newState);
      return newState;
    });
  },

  setActiveTab: (id) => {
    set({ activeTabId: id });
    broadcastStateChange('active_tab_changed', { activeTabId: id });
  },

  updateTabTitle: (id, title) => {
    set((state) => {
      const newTabs = state.tabs.map((t) => (t.id === id ? { ...t, title } : t));
      const newTerminals = state.terminals.map((t) => (t.tabId === id ? { ...t, title } : t));
      const newState = { tabs: newTabs, terminals: newTerminals };
      broadcastStateChange('tab_updated', newState);
      return newState;
    });
  },

  addTerminal: (terminal) => {
    const id = generateId();
    const newTerminal: TerminalInstance = { ...terminal, id };
    set((state) => {
      const newState = { terminals: [...state.terminals, newTerminal] };
      broadcastStateChange('tab_updated', { terminals: newState.terminals });
      return newState;
    });
    return id;
  },

  removeTerminal: (id) => {
    set((state) => {
      const newState = { terminals: state.terminals.filter((t) => t.id !== id) };
      broadcastStateChange('tab_updated', newState);
      return newState;
    });
  },

  setGridLayout: (layout) => {
    set({ gridLayout: layout });
    broadcastStateChange('layout_changed', { gridLayout: layout });
  },

  setFullscreen: (isFullscreen) => {
    set({ isFullscreen });
    broadcastStateChange('fullscreen_changed', { isFullscreen });
  },

  appendLog: (terminalId, log) => {
    set((state) => {
      const newLogs = new Map(state.logs);
      const existing = newLogs.get(terminalId) || [];
      newLogs.set(terminalId, [...existing, log]);
      return { logs: newLogs };
    });
    // 广播到其他窗口
    broadcastOutput(terminalId, log);
  },

  // 接收来自其他窗口的日志（不重新广播）
  receiveLog: (terminalId, log) => {
    set((state) => {
      const newLogs = new Map(state.logs);
      const existing = newLogs.get(terminalId) || [];
      newLogs.set(terminalId, [...existing, log]);
      return { logs: newLogs };
    });
  },

  clearLogs: (terminalId) => {
    set((state) => {
      const newLogs = new Map(state.logs);
      newLogs.set(terminalId, []);
      return { logs: newLogs };
    });
  },

  // 同步状态更新（用于处理来自其他窗口的同步消息）
  syncState: (newState: Partial<TerminalState>) => {
    set((state) => ({
      ...newState,
      // logs 保持不变，不同步日志
      logs: state.logs,
    }));
  },
}));
