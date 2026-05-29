import { useEffect, useRef, useState, useCallback } from 'react';
import { Allotment } from 'allotment';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Plus, X, Maximize2, Grid, Rows, Square } from 'lucide-react';
import { useTerminalStore } from '@/stores/terminalStore';
import { useShellPtySession } from '@/hooks/useShellPtySession';
import { useThemeStore } from '@/stores/themeStore';
import { xtermThemeFor } from '@/components/terminal/xtermThemes';
import { cn } from '@/lib/utils';

import '@xterm/xterm/css/xterm.css';

// 布局配置映射
const GRID_LAYOUTS = {
  '1x1': { columns: 1, rows: 1 },
  '2x1': { columns: 2, rows: 1 },
  '2x2': { columns: 2, rows: 2 },
  '3x3': { columns: 3, rows: 3 },
} as const;

type GridLayoutType = keyof typeof GRID_LAYOUTS;

// 单个终端面板 — 原生 PTY raw 模式（Tauri 本地 / 浏览器 WS 二进制）
function TerminalPane({
  title,
  onClose,
  initialCwd,
  compact = false,
  panelHeight,
  visible = true,
}: {
  terminalId: string;
  title: string;
  onClose?: () => void;
  initialCwd?: string;
  compact?: boolean;
  panelHeight?: number;
  visible?: boolean;
}) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [terminalReady, setTerminalReady] = useState(false);
  const [sessionKey, setSessionKey] = useState(0);
  const resolvedTheme = useThemeStore((s) => s.resolvedTheme);

  const { isConnected, sessionEnded, resize } = useShellPtySession({
    terminal: terminalReady ? xtermRef.current : null,
    cwd: initialCwd,
    enabled: terminalReady,
    sessionKey,
    onSessionEnded: () => {},
  });

  useEffect(() => {
    if (!terminalRef.current || xtermRef.current) return;

    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: compact ? 12 : 14,
      lineHeight: 1.2,
      scrollback: 5000,
      convertEol: false,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: xtermThemeFor(useThemeStore.getState().resolvedTheme),
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalRef.current);

    xtermRef.current = terminal;
    fitAddonRef.current = fitAddon;
    setTerminalReady(true);

    const fitTerminal = () => {
      try {
        const host = terminalRef.current;
        if (!host || host.clientWidth < 8 || host.clientHeight < 8) return;
        fitAddon.fit();
        resize(terminal.rows, terminal.cols);
      } catch {
        /* ignore */
      }
    };

    requestAnimationFrame(fitTerminal);
    [50, 120, 300, 600].forEach((ms) => window.setTimeout(fitTerminal, ms));

    const resizeDisposable = terminal.onResize(({ rows, cols }) => {
      resize(rows, cols);
    });

    const resizeObserver = new ResizeObserver(fitTerminal);
    resizeObserver.observe(terminalRef.current);
    if (terminalRef.current.parentElement) {
      resizeObserver.observe(terminalRef.current.parentElement);
    }

    return () => {
      resizeObserver.disconnect();
      resizeDisposable.dispose();
      terminal.dispose();
      xtermRef.current = null;
      fitAddonRef.current = null;
      setTerminalReady(false);
    };
  }, [compact, resize]);

  useEffect(() => {
    const term = xtermRef.current;
    if (!term) return;
    term.options.theme = xtermThemeFor(resolvedTheme);
    term.refresh(0, term.rows - 1);
  }, [resolvedTheme]);

  useEffect(() => {
    if (!visible) return;
    const fit = () => {
      try {
        const host = terminalRef.current;
        if (!host || host.clientWidth < 8 || host.clientHeight < 8) return;
        fitAddonRef.current?.fit();
        const term = xtermRef.current;
        if (term) resize(term.rows, term.cols);
        term?.focus();
      } catch {
        /* ignore */
      }
    };
    requestAnimationFrame(fit);
    const ids = [50, 120, 300, 600].map((ms) => window.setTimeout(fit, ms));
    return () => ids.forEach((id) => window.clearTimeout(id));
  }, [visible, panelHeight, resize]);

  useEffect(() => {
    if (!panelHeight) return;
    try {
      fitAddonRef.current?.fit();
      const term = xtermRef.current;
      if (term) resize(term.rows, term.cols);
    } catch {
      /* ignore */
    }
  }, [panelHeight, resize]);

  const handleReconnect = useCallback(() => {
    xtermRef.current?.clear();
    setSessionKey((k) => k + 1);
  }, []);

  return (
    <div
      className={cn(
        'h-full min-h-0 flex flex-col overflow-hidden',
        compact ? 'bg-transparent' : 'bg-background rounded-md border border-border',
      )}
    >
      {!compact && (
        <>
          <div className="flex items-center justify-between px-3 py-1.5 bg-muted/50 border-b border-border shrink-0">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground truncate">{title}</span>
              <span
                className={cn(
                  'w-2 h-2 rounded-full transition-colors',
                  isConnected ? 'bg-green-500' : sessionEnded ? 'bg-red-500' : 'bg-yellow-500 animate-pulse',
                )}
              />
            </div>
            {onClose && (
              <button
                onClick={onClose}
                className="p-0.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </>
      )}

      <div
        className="flex-1 min-h-0 relative overflow-hidden"
        onClick={() => xtermRef.current?.focus()}
      >
        {sessionEnded && (
          <div className="absolute inset-x-0 top-0 z-20 flex items-center justify-between gap-2 px-3 py-2 bg-amber-500/10 border-b border-amber-500/30 text-xs">
            <span className="text-amber-800 dark:text-amber-200/90 truncate">Shell 已退出</span>
            <button
              type="button"
              className="shrink-0 px-2 py-1 rounded bg-primary/90 text-primary-foreground hover:bg-primary"
              onClick={handleReconnect}
            >
              重新连接
            </button>
          </div>
        )}
        {!isConnected && !sessionEnded && compact && (
          <div className="absolute inset-0 z-10 flex items-center justify-center pointer-events-none">
            <span className="text-xs text-muted-foreground">连接 Shell…</span>
          </div>
        )}
        <div ref={terminalRef} className="absolute inset-0 xterm-host" tabIndex={0} />
      </div>
    </div>
  );
}

// 布局切换按钮组
function LayoutSwitcher({
  currentLayout,
  onLayoutChange,
}: {
  currentLayout: GridLayoutType;
  onLayoutChange: (layout: GridLayoutType) => void;
}) {
  const layouts: { id: GridLayoutType; icon: React.ReactNode; label: string }[] = [
    { id: '1x1', icon: <Square className="w-4 h-4" />, label: '1x1' },
    { id: '2x1', icon: <Rows className="w-4 h-4" />, label: '2x1' },
    { id: '2x2', icon: <Grid className="w-4 h-4" />, label: '2x2' },
    { id: '3x3', icon: <Grid className="w-4 h-4" />, label: '3x3' },
  ];

  return (
    <div className="flex items-center gap-1">
      {layouts.map(({ id, icon, label }) => (
        <button
          key={id}
          onClick={() => onLayoutChange(id)}
          className={cn(
            'p-1.5 rounded transition-colors',
            currentLayout === id
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-accent text-muted-foreground',
          )}
          title={label}
        >
          {icon}
        </button>
      ))}
    </div>
  );
}

// 标签页栏
function TabBar() {
  const { tabs, activeTabId, setActiveTab, addTab, removeTab } = useTerminalStore();

  const handleAddTab = () => {
    const newTabId = addTab({
      title: `终端 ${tabs.length + 1}`,
    });
    // 为新标签创建默认终端
    useTerminalStore.getState().addTerminal({
      tabId: newTabId,
      title: `终端 ${tabs.length + 1}`,
    });
    setActiveTab(newTabId);
  };

  return (
    <div className="flex items-center gap-1 px-2 py-1.5 bg-card border-b">
      <div className="flex items-center gap-1 overflow-x-auto flex-1">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={cn(
              'flex items-center gap-2 px-3 py-1 rounded-md text-sm cursor-pointer transition-colors group',
              activeTabId === tab.id ? 'bg-primary text-primary-foreground' : 'hover:bg-accent',
            )}
            onClick={() => setActiveTab(tab.id)}
          >
            <span className="truncate max-w-[120px]">{tab.title}</span>
            {tabs.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeTab(tab.id);
                }}
                className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-black/20"
              >
                <X className="w-3 h-3" />
              </button>
            )}
          </div>
        ))}
      </div>
      <button
        onClick={handleAddTab}
        className="p-1 rounded hover:bg-accent text-muted-foreground"
        title="新建标签"
      >
        <Plus className="w-4 h-4" />
      </button>
    </div>
  );
}

// 主终端网格组件
export function TerminalGrid({
  initialCwd,
  compact = false,
  integrated = false,
  panelHeight,
  visible = true,
}: {
  initialCwd?: string;
  compact?: boolean;
  integrated?: boolean;
  panelHeight?: number;
  visible?: boolean;
} = {}) {
  const { gridLayout, setGridLayout, terminals, activeTabId, isFullscreen, setFullscreen } =
    useTerminalStore();

  // 根据活动标签页筛选终端
  const activeTerminals = terminals.filter((t) => t.tabId === activeTabId);

  // 获取当前布局配置
  const layoutConfig = GRID_LAYOUTS[gridLayout as GridLayoutType] || GRID_LAYOUTS['2x2'];
  const totalPanes = layoutConfig.columns * layoutConfig.rows;

  // 切换全屏模式
  const toggleFullscreen = useCallback(() => {
    setFullscreen(!isFullscreen);
  }, [isFullscreen, setFullscreen]);

  const shellCompact = compact || integrated;

  return (
    <div
      className={cn(
        'flex flex-col overflow-hidden h-full min-h-0',
        !shellCompact && 'bg-card border rounded-lg',
        isFullscreen && 'fixed inset-4 z-50 bg-card border rounded-lg',
        shellCompact && 'border-0 rounded-none bg-transparent',
      )}
    >
      {!shellCompact && (
        <>
          {/* 工具栏 */}
          <div className="flex items-center justify-between px-3 py-2 bg-card border-b">
            <div className="flex items-center gap-3">
              <h3 className="text-sm font-medium">终端</h3>
              <LayoutSwitcher
                currentLayout={gridLayout as GridLayoutType}
                onLayoutChange={setGridLayout}
              />
            </div>
            <button
              onClick={toggleFullscreen}
              className="p-1.5 rounded hover:bg-accent text-muted-foreground"
              title={isFullscreen ? '退出全屏' : '全屏'}
            >
              <Maximize2 className="w-4 h-4" />
            </button>
          </div>

          {/* 标签页栏 */}
          <TabBar />
        </>
      )}

      {/* 终端网格 */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {shellCompact ? (
          activeTerminals[0] ? (
            <TerminalPane
              terminalId={activeTerminals[0].id}
              title={activeTerminals[0].title}
              initialCwd={initialCwd}
              compact
              panelHeight={panelHeight}
              visible={visible}
            />
          ) : (
            <div className="h-full flex items-center justify-center bg-background">
              <div className="text-muted-foreground text-sm">暂无终端 — 点击 + 新建</div>
            </div>
          )
        ) : (
          <Allotment>
            {Array.from({ length: totalPanes }).map((_, index) => {
              const terminal = activeTerminals[index];
              return (
                <Allotment.Pane key={terminal?.id || `empty-${index}`} minSize={100}>
                  {terminal ? (
                    <TerminalPane
                      terminalId={terminal.id}
                      title={terminal.title}
                      initialCwd={initialCwd}
                    />
                  ) : (
                    <div className="h-full flex items-center justify-center bg-[#1e1e1e]">
                      <div className="text-gray-500 text-sm">暂无终端 - 点击 + 添加</div>
                    </div>
                  )}
                </Allotment.Pane>
              );
            })}
          </Allotment>
        )}
      </div>
    </div>
  );
}
