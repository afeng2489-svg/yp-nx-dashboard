import { useCallback, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { ExternalLink, Globe, GripHorizontal, RefreshCw, Terminal, X, FileCode } from 'lucide-react';
import { TerminalGrid } from '@/components/terminal';
import {
  clampFactoryDrawerHeight,
  useFactoryDrawerStore,
} from '@/stores/factoryDrawerStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { cn } from '@/lib/utils';
import { useIsStudioDark } from '@/components/layout/ShellThemeContext';

const TABS = [
  { id: 'terminal' as const, label: '终端', icon: Terminal },
  { id: 'editor' as const, label: '编辑器', icon: FileCode },
  { id: 'browser' as const, label: '浏览器', icon: Globe },
];

export function FactoryDrawer() {
  const isStudio = useIsStudioDark();
  const navigate = useNavigate();
  const {
    isOpen,
    activeTab,
    contentHeight,
    sessionEpoch,
    terminalEverOpened,
    close,
    setTab,
    setContentHeight,
    resetTerminalSession,
  } = useFactoryDrawerStore();
  const openFiles = useWorkspaceStore((s) => s.openFiles);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const dragRef = useRef<{ startY: number; startHeight: number } | null>(null);
  const lastWorkspaceIdRef = useRef<string | null>(null);

  // 切换工作区时重建终端（cwd 变了）
  useEffect(() => {
    const id = currentWorkspace?.id ?? null;
    if (lastWorkspaceIdRef.current !== null && id !== lastWorkspaceIdRef.current) {
      resetTerminalSession();
    }
    lastWorkspaceIdRef.current = id;
  }, [currentWorkspace?.id, resetTerminalSession]);

  const onResizeStart = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      dragRef.current = { startY: event.clientY, startHeight: contentHeight };

      const onMove = (ev: MouseEvent) => {
        if (!dragRef.current) return;
        const delta = dragRef.current.startY - ev.clientY;
        setContentHeight(clampFactoryDrawerHeight(dragRef.current.startHeight + delta));
      };

      const onUp = () => {
        dragRef.current = null;
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };

      document.body.style.cursor = 'ns-resize';
      document.body.style.userSelect = 'none';
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    },
    [contentHeight, setContentHeight],
  );

  useEffect(() => {
    if (!isOpen) return;
    const onWindowResize = () => {
      setContentHeight(clampFactoryDrawerHeight(useFactoryDrawerStore.getState().contentHeight));
    };
    window.addEventListener('resize', onWindowResize);
    return () => window.removeEventListener('resize', onWindowResize);
  }, [isOpen, setContentHeight]);

  // 从未打开过时不挂载（避免后台空连 WS）
  if (!terminalEverOpened && !isOpen) return null;

  const isDev = import.meta.env.DEV;
  const terminalVisible = isOpen && activeTab === 'terminal';

  return (
    <div
      className={cn(
        'fixed inset-x-0 bottom-7 z-40 flex flex-col border-t shadow-2xl',
        isOpen && 'animate-slide-up',
        !isOpen && 'hidden',
        isStudio
          ? 'border-border bg-background text-foreground'
          : 'border-border/60 bg-card',
      )}
      aria-hidden={!isOpen}
    >
      <button
        type="button"
        aria-label="拖动调整终端高度"
        onMouseDown={onResizeStart}
        tabIndex={isOpen ? 0 : -1}
        className={cn(
          'group flex h-2.5 shrink-0 cursor-ns-resize items-center justify-center border-b transition-colors',
          isStudio
            ? 'border-border bg-muted/40 hover:bg-muted/70'
            : 'border-border/50 bg-muted/30 hover:bg-muted/50',
        )}
      >
        <GripHorizontal className="h-3 w-3 text-muted-foreground/60 group-hover:text-muted-foreground" />
      </button>

      <div
        className={cn(
          'flex items-center justify-between px-3 py-1.5 border-b shrink-0',
          isStudio ? 'border-border bg-muted/80' : 'border-border/50 bg-muted/30',
        )}
      >
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <div className="flex gap-1 shrink-0">
            {TABS.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                type="button"
                onClick={() => setTab(id)}
                tabIndex={isOpen ? 0 : -1}
                className={cn(
                  'inline-flex items-center gap-1.5 px-3 py-1 text-xs rounded-md transition-colors',
                  activeTab === id
                    ? isStudio
                      ? 'bg-accent text-foreground font-medium'
                      : 'bg-primary/10 text-primary font-medium'
                    : isStudio
                      ? 'text-muted-foreground hover:text-foreground hover:bg-accent/60'
                      : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                )}
              >
                <Icon className="w-3.5 h-3.5" />
                {label}
              </button>
            ))}
          </div>
          {activeTab === 'terminal' && currentWorkspace?.root_path && (
            <span
              className="hidden sm:block text-[10px] font-mono text-muted-foreground truncate min-w-0"
              title={currentWorkspace.root_path}
            >
              {currentWorkspace.root_path}
            </span>
          )}
          {activeTab === 'terminal' && (
            <button
              type="button"
              onClick={resetTerminalSession}
              tabIndex={isOpen ? 0 : -1}
              className="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] rounded-md border border-border/50 hover:bg-accent/50 text-muted-foreground shrink-0"
              title="重新连接 Shell"
            >
              <RefreshCw className="w-3 h-3" />
              新会话
            </button>
          )}
        </div>
        <button
          type="button"
          onClick={close}
          tabIndex={isOpen ? 0 : -1}
          className="p-1.5 rounded-md hover:bg-accent shrink-0"
          title="关闭（会话在后台保持，再打开可继续）"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="overflow-hidden shrink-0" style={{ height: contentHeight }}>
        {/* 终端常驻挂载：关抽屉只隐藏，不杀 PTY — 未按回车的输入仍留在 Shell 行缓冲 */}
        <div
          className={cn(
            'factory-drawer-terminal h-full min-h-0 flex flex-col bg-background',
            !terminalVisible && 'hidden',
          )}
        >
          <div className="flex-1 min-h-0 overflow-hidden">
            <TerminalGrid
              key={sessionEpoch}
              initialCwd={currentWorkspace?.root_path}
              compact
              panelHeight={contentHeight}
              visible={terminalVisible}
            />
          </div>
        </div>

        {activeTab === 'editor' && isOpen && (
          <div className="h-full flex flex-col items-center justify-center gap-3 p-6 text-sm text-muted-foreground">
            <FileCode className="w-8 h-8 opacity-50" />
            {openFiles.length > 0 ? (
              <p>已在主区域打开 {openFiles.length} 个文件 — 使用左侧文件树或顶栏切换</p>
            ) : (
              <p>从产物 Tab 或文件树打开文件后，编辑器会出现在主内容区</p>
            )}
            {currentWorkspace?.root_path && (
              <p className="text-xs font-mono truncate max-w-full" title={currentWorkspace.root_path}>
                工作区: {currentWorkspace.root_path}
              </p>
            )}
          </div>
        )}

        {activeTab === 'browser' && isOpen && (
          <div className="h-full flex flex-col min-h-0">
            <div className="flex items-center justify-between px-4 py-2 border-b border-border/40 text-xs shrink-0">
              <span className="text-muted-foreground">Tauri 环境不支持内嵌 webview</span>
              <button
                type="button"
                onClick={() => {
                  navigate('/browser');
                  close();
                }}
                className="inline-flex items-center gap-1 text-primary hover:underline"
              >
                <ExternalLink className="w-3.5 h-3.5" />
                在新窗口打开
              </button>
            </div>
            {isDev ? (
              <iframe
                title="browser-fallback"
                src="about:blank"
                className="flex-1 min-h-0 w-full bg-background"
                sandbox="allow-scripts allow-same-origin"
              />
            ) : (
              <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
                点击上方按钮在外部浏览器窗口打开
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
