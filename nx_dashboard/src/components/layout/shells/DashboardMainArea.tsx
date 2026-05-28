import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import { Outlet } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { ContextPanel } from '@/components/factory/ContextPanel';
import { FileEditor } from '@/components/editor/FileEditor';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { ShellSurface } from '@/components/layout/ShellSurface';
import { IntegratedTerminalPanel } from '@/components/terminal/IntegratedTerminalPanel';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';

interface DashboardMainAreaProps {
  showFileSidebar: boolean;
  fileSidebar: React.ReactNode;
  showContextPanel?: boolean;
  outletClassName?: string;
  /** 工作室：Cursor 式底部集成终端（Allotment 分割） */
  integratedTerminal?: boolean;
}

function MainRow({
  showFileSidebar,
  fileSidebar,
  showContextPanel,
  outletClassName,
}: Omit<DashboardMainAreaProps, 'integratedTerminal'>) {
  const openFiles = useWorkspaceStore((s) => s.openFiles);
  const hasOpenFiles = openFiles.length > 0;
  const contextPanelOpen = useContextPanelStore((s) => s.isOpen);

  return (
    <div className="flex h-full min-h-0 overflow-hidden">
      {showFileSidebar && fileSidebar}

      <main className="flex-1 overflow-hidden min-w-0">
        {hasOpenFiles ? (
          <Allotment vertical defaultSizes={[60, 40]}>
            <Allotment.Pane minSize={200}>
              <FileEditor />
            </Allotment.Pane>
            <Allotment.Pane minSize={100}>
              <ShellSurface className={cn('h-full overflow-auto', outletClassName)}>
                <Outlet />
              </ShellSurface>
            </Allotment.Pane>
          </Allotment>
        ) : (
          <ShellSurface className={cn('h-full overflow-auto', outletClassName)}>
            <Outlet />
          </ShellSurface>
        )}
      </main>

      {showContextPanel && contextPanelOpen && !hasOpenFiles && <ContextPanel />}
    </div>
  );
}

/** 主内容区：可选文件树 + 编辑器分屏 + 页面 Outlet + ContextPanel */
export function DashboardMainArea({
  showFileSidebar,
  fileSidebar,
  showContextPanel = true,
  outletClassName,
  integratedTerminal = false,
}: DashboardMainAreaProps) {
  const integratedVisible = useFactoryDrawerStore((s) => s.integratedVisible);
  const terminalEverOpened = useFactoryDrawerStore((s) => s.terminalEverOpened);

  const row = (
    <MainRow
      showFileSidebar={showFileSidebar}
      fileSidebar={fileSidebar}
      showContextPanel={showContextPanel}
      outletClassName={outletClassName}
    />
  );

  if (!integratedTerminal) {
    return <div className="flex-1 flex overflow-hidden min-w-0">{row}</div>;
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
      <Allotment vertical>
        <Allotment.Pane minSize={120}>{row}</Allotment.Pane>
        {terminalEverOpened && (
          <Allotment.Pane
            visible={integratedVisible}
            preferredSize={280}
            minSize={80}
            maxSize={480}
          >
            <IntegratedTerminalPanel visible={integratedVisible} />
          </Allotment.Pane>
        )}
      </Allotment>
    </div>
  );
}
