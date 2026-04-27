import { useState } from 'react';
import { Sidebar } from './Sidebar';
import { WorkspaceSelector } from '@/components/workspace/WorkspaceSelector';
import { FileSidebar } from '@/components/explorer/FileSidebar';
import { FileEditor } from '@/components/editor/FileEditor';
import { Outlet } from 'react-router-dom';
import { PanelLeftClose, PanelLeft } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import { GlobalOpsOverlay } from '@/components/global/GlobalOpsOverlay';

export function Dashboard() {
  const [showFileSidebar, setShowFileSidebar] = useState(true);
  const openFiles = useWorkspaceStore((s) => s.openFiles);
  const hasOpenFiles = openFiles.length > 0;

  return (
    <div className="flex h-screen">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header with Workspace Selector */}
        <header className="relative z-50 h-14 px-6 flex items-center justify-between border-b border-border/50 bg-card/50 backdrop-blur-sm">
          <div className="flex items-center gap-4">
            <WorkspaceSelector />
            <button
              onClick={() => setShowFileSidebar(!showFileSidebar)}
              className={cn(
                'p-2 rounded-lg hover:bg-accent transition-colors',
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

        {/* Main content area with optional file sidebar */}
        <div className="flex-1 flex overflow-hidden">
          {/* File Sidebar - collapsible */}
          {showFileSidebar && (
            <div className="w-64 border-r border-border overflow-hidden">
              <FileSidebar />
            </div>
          )}

          {/* Page content — split when editor is open */}
          <main className="flex-1 overflow-hidden">
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
