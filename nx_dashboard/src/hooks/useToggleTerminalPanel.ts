import { useEffect } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';

/** Cursor 风格：⌃` / ⌘` 切换底部终端面板（工作室）或工具抽屉（引导） */
export function useToggleTerminalPanel() {
  const layoutMode = useSettingsStore((s) => s.layout.mode);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== '`' && event.code !== 'Backquote') return;
      if (!event.ctrlKey && !event.metaKey) return;
      if (event.altKey) return;

      event.preventDefault();
      event.stopPropagation();

      if (layoutMode === 'studio') {
        useFactoryDrawerStore.getState().toggleIntegrated();
      } else {
        const { isOpen, open, close } = useFactoryDrawerStore.getState();
        if (isOpen) close();
        else open('terminal');
      }
    };

    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [layoutMode]);
}
