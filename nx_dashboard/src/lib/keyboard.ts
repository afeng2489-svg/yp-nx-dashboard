import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface KeyboardShortcut {
  key: string;
  modifiers?: ('ctrl' | 'alt' | 'shift' | 'meta')[];
  description: string;
  action: () => void;
  scope?: 'global' | 'editor' | 'terminal';
}

interface KeyboardStore {
  shortcuts: Map<string, KeyboardShortcut>;
  isEnabled: boolean;
  registerShortcut: (id: string, shortcut: KeyboardShortcut) => void;
  unregisterShortcut: (id: string) => void;
  setEnabled: (enabled: boolean) => void;
  getShortcutDisplay: (id: string) => string;
}

// Format shortcut for display
function formatShortcut(shortcut: KeyboardShortcut): string {
  const parts: string[] = [];

  if (shortcut.modifiers) {
    const modMap: Record<string, string> = {
      ctrl: 'Ctrl',
      alt: 'Alt',
      shift: 'Shift',
      meta: '⌘',
    };
    shortcut.modifiers.forEach((m) => parts.push(modMap[m]));
  }

  // Format key
  let key = shortcut.key;
  const specialKeys: Record<string, string> = {
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Enter: '↵',
    Escape: 'Esc',
    Backspace: '⌫',
    Delete: 'Del',
  };

  if (specialKeys[key]) {
    key = specialKeys[key];
  } else if (key.length === 1) {
    key = key.toUpperCase();
  }

  parts.push(key);
  return parts.join('+');
}

export const useKeyboardStore = create<KeyboardStore>()(
  persist(
    (set, get) => ({
      shortcuts: new Map(),
      isEnabled: true,

      registerShortcut: (id, shortcut) => {
        set((state) => {
          const newShortcuts = new Map(state.shortcuts);
          newShortcuts.set(id, shortcut);
          return { shortcuts: newShortcuts };
        });
      },

      unregisterShortcut: (id) => {
        set((state) => {
          const newShortcuts = new Map(state.shortcuts);
          newShortcuts.delete(id);
          return { shortcuts: newShortcuts };
        });
      },

      setEnabled: (enabled) => set({ isEnabled: enabled }),

      getShortcutDisplay: (id) => {
        const shortcut = get().shortcuts.get(id);
        return shortcut ? formatShortcut(shortcut) : '';
      },
    }),
    {
      name: 'nexusflow-keyboard',
      partialize: (state) => ({ isEnabled: state.isEnabled }),
    },
  ),
);

// Hook to use a specific shortcut
export function useShortcut(id: string): KeyboardShortcut | undefined {
  return useKeyboardStore((state) => state.shortcuts.get(id));
}

// Hook to register a shortcut on mount
import { useEffect } from 'react';

export function useRegisterShortcut(id: string, shortcut: KeyboardShortcut, deps: unknown[] = []) {
  const register = useKeyboardStore((state) => state.registerShortcut);
  const unregister = useKeyboardStore((state) => state.unregisterShortcut);

  useEffect(() => {
    register(id, shortcut);

    return () => {
      unregister(id);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, ...deps]);

  return useShortcut(id);
}

// Build keyboard event matcher
export function matchesShortcut(event: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
  const key = event.key;

  // Check modifiers
  if (shortcut.modifiers) {
    const requiresCtrl = shortcut.modifiers.includes('ctrl');
    const requiresAlt = shortcut.modifiers.includes('alt');
    const requiresShift = shortcut.modifiers.includes('shift');
    const requiresMeta = shortcut.modifiers.includes('meta');

    if (requiresCtrl !== event.ctrlKey) return false;
    if (requiresAlt !== event.altKey) return false;
    if (requiresShift !== event.shiftKey) return false;
    if (requiresMeta !== event.metaKey) return false;
  }

  // Check key (case insensitive)
  return key.toLowerCase() === shortcut.key.toLowerCase();
}

// Global keyboard handler
export function useKeyboardHandler() {
  const { shortcuts, isEnabled } = useKeyboardStore();

  useEffect(() => {
    if (!isEnabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Skip if in input/textarea
      const target = event.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      // Find matching shortcut
      for (const shortcut of shortcuts.values()) {
        if (matchesShortcut(event, shortcut)) {
          event.preventDefault();
          shortcut.action();
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [shortcuts, isEnabled]);
}

// Built-in shortcut IDs
export const SHORTCUT_IDS = {
  // Global
  TOGGLE_SIDEBAR: 'global:toggle-sidebar',
  TOGGLE_THEME: 'global:toggle-theme',
  OPEN_COMMAND_PALETTE: 'global:command-palette',
  GO_TO_DASHBOARD: 'global:go-dashboard',
  GO_TO_WORKFLOWS: 'global:go-workflows',
  GO_TO_EXECUTIONS: 'global:go-executions',
  GO_TO_TERMINAL: 'global:go-terminal',
  GO_TO_SESSIONS: 'global:go-sessions',

  // Editor
  SAVE_WORKFLOW: 'editor:save',
  DELETE_NODE: 'editor:delete-node',
  UNDO: 'editor:undo',
  REDO: 'editor:redo',
  ZOOM_IN: 'editor:zoom-in',
  ZOOM_OUT: 'editor:zoom-out',
  FIT_VIEW: 'editor:fit-view',
} as const;
