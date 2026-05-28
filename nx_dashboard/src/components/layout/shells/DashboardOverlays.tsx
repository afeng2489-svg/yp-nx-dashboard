import { FactoryDrawer } from '@/components/factory/FactoryDrawer';
import { GlobalOpsOverlay } from '@/components/global/GlobalOpsOverlay';
import { GlobalHelpButton } from '@/components/guide/GlobalHelpButton';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToggleTerminalPanel } from '@/hooks/useToggleTerminalPanel';

/** 全局浮层：引导模式工具抽屉、运维 overlay、帮助按钮 */
export function DashboardOverlays() {
  const layoutMode = useSettingsStore((s) => s.layout.mode);
  useToggleTerminalPanel();

  return (
    <>
      {layoutMode !== 'studio' && <FactoryDrawer />}
      <GlobalOpsOverlay />
      <GlobalHelpButton />
    </>
  );
}
