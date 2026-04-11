import { useState } from 'react';
import { Sidebar } from './Sidebar';
import { WorkspaceSelector } from '@/components/workspace/WorkspaceSelector';
import { FileSidebar } from '@/components/explorer/FileSidebar';
import { Outlet } from 'react-router-dom';
import { PanelLeftClose, PanelLeft } from 'lucide-react';
import { cn } from '@/lib/utils';

export function Dashboard() {
  const [showFileSidebar, setShowFileSidebar] = useState(true);

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
                showFileSidebar && 'bg-accent'
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

          {/* Page content */}
          <main className="flex-1 overflow-auto">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}