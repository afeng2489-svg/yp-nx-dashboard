import { useState } from 'react';
import { Sidebar } from './Sidebar';
import { WorkspaceSelector } from '@/components/workspace/WorkspaceSelector';
import { FileSidebar } from '@/components/explorer/FileSidebar';
import { FileEditor } from '@/components/editor/FileEditor';
import { Outlet } from 'react-router-dom';
import { PanelLeftClose, PanelLeft } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useUIStore } from '@/stores/uiStore';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import { GlobalOpsOverlay } from '@/components/global/GlobalOpsOverlay';
import { ClaudeCliMissingBanner } from '@/components/global/ClaudeCliMissingBanner';
import { useShrinkBelow } from '@/hooks/useResponsive';

export function Dashboard() {
  const [showFileSidebar, setShowFileSidebar] = useState(true);
  const openFiles = useWorkspaceStore((s) => s.openFiles);
  const hasOpenFiles = openFiles.length > 0;
  const setSidebarOpen = useUIStore((s) => s.setSidebarOpen);

  // 窄屏自动收起左侧导航栏（< 1024px）
  useShrinkBelow(1024, () => setSidebarOpen(false));

  // 更窄屏自动隐藏文件浏览器（< 1280px），让主内容有足够空间
  useShrinkBelow(1280, () => setShowFileSidebar(false));

  return (
    <div className="flex h-screen min-w-0">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        {/* Header with Workspace Selector */}
        <header className="relative z-50 h-14 px-4 sm:px-6 flex items-center justify-between border-b border-border/50 bg-card/50 backdrop-blur-sm">
          <div className="flex items-center gap-2 sm:gap-4 min-w-0">
            <WorkspaceSelector />
            <button
              onClick={() => setShowFileSidebar(!showFileSidebar)}
              className={cn(
                'p-2 rounded-lg hover:bg-accent transition-colors flex-shrink-0',
                showFileSidebar && 'bg-accent',
              )}
              title={showFileSidebar ? '隐藏文件浏览器' : '显示文件浏览器'}
            >
              {showFileSidebar ? (
                <PanelLeftClose className="w-4 h-4" />
              ) : (
                <PanelLeft className="w-4 h-4" />
              )}
            </button>
          </div>
        </header>

        {/* Claude CLI 缺失警告（仅未检测到时显示） */}
        <ClaudeCliMissingBanner />

        {/* Main content area with optional file sidebar */}
        <div className="flex-1 flex overflow-hidden min-w-0">
          {/* File Sidebar - collapsible */}
          {showFileSidebar && (
            <div className="w-56 lg:w-64 border-r border-border overflow-hidden flex-shrink-0">
              <FileSidebar />
            </div>
          )}

          {/* Page content — split when editor is open */}
          <main className="flex-1 overflow-hidden min-w-0">
            {hasOpenFiles ? (
              <Allotment vertical defaultSizes={[60, 40]}>
                <Allotment.Pane minSize={200}>
                  <FileEditor />
                </Allotment.Pane>
                <Allotment.Pane minSize={100}>
                  <div className="h-full overflow-auto">
                    <Outlet />
                  </div>
                </Allotment.Pane>
              </Allotment>
            ) : (
              <div className="h-full overflow-auto">
                <Outlet />
              </div>
            )}
          </main>
        </div>
      </div>
      <GlobalOpsOverlay />
    </div>
  );
}
