import { useEffect, useRef, useState, useCallback } from 'react';
import { Allotment } from 'allotment';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Plus, X, Maximize2, Grid, Rows, Square } from 'lucide-react';
import { useTerminalStore } from '@/stores/terminalStore';
import { cn } from '@/lib/utils';
import { WS_BASE_URL } from '@/api/constants';

import '@xterm/xterm/css/xterm.css';

// 布局配置映射
const GRID_LAYOUTS = {
  '1x1': { columns: 1, rows: 1 },
  '2x1': { columns: 2, rows: 1 },
  '2x2': { columns: 2, rows: 2 },
  '3x3': { columns: 3, rows: 3 },
} as const;

type GridLayoutType = keyof typeof GRID_LAYOUTS;

// WebSocket 连接状态
interface WsConnection {
  ws: WebSocket | null;
  terminal: Terminal | null;
  reconnectAttempts: number;
  reconnectTimeout: ReturnType<typeof setTimeout> | null;
}

// WebSocket 重连配置
const WS_RECONNECT_CONFIG = {
  maxAttempts: 5,
  baseDelay: 1000,
  maxDelay: 30000,
  backoffMultiplier: 2,
};

// 单个终端面板
function TerminalPane({
  terminalId: _terminalId,
  title,
  onClose,
}: {
  terminalId: string;
  title: string;
  onClose?: () => void;
}) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WsConnection>({ ws: null, terminal: null, reconnectAttempts: 0, reconnectTimeout: null });
  const [isLoading, setIsLoading] = useState(true);
  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    // 初始化 xterm
    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#ffffff',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78',
        black: '#1e1e1e',
        red: '#f44747',
        green: '#6a9955',
        yellow: '#dcdcaa',
        blue: '#569cd6',
        magenta: '#c586c0',
        cyan: '#4ec9b0',
        white: '#d4d4d4',
        brightBlack: '#808080',
        brightRed: '#f44747',
        brightGreen: '#6a9955',
        brightYellow: '#dcdcaa',
        brightBlue: '#569cd6',
        brightMagenta: '#c586c0',
        brightCyan: '#4ec9b0',
        brightWhite: '#ffffff',
      },
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(terminalRef.current);
    // Delay fit() to allow the terminal to render and calculate dimensions
    requestAnimationFrame(() => {
      fitAddon.fit();
    });

    xtermRef.current = terminal;
    fitAddonRef.current = fitAddon;
    wsRef.current.terminal = terminal;

    // 建立 WebSocket 连接的函数
    const connectWebSocket = () => {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${WS_BASE_URL}/ws/terminal`;

      try {
        const ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          setIsConnected(true);
          setConnectionError(null);
          wsRef.current.reconnectAttempts = 0;
          terminal.writeln('\x1b[36m[NexusFlow]\x1b[0m 终端已连接');
          terminal.writeln('');
        };

        ws.onmessage = (event) => {
          try {
            const msg = JSON.parse(event.data);
            if (msg.type === 'output') {
              terminal.write(msg.data);
            } else if (msg.type === 'error') {
              terminal.writeln(`\x1b[31m[错误]\x1b[0m ${msg.message}`);
            }
          } catch {
            // 原始文本输出
            terminal.write(event.data);
          }
        };

        ws.onclose = () => {
          setIsConnected(false);
          terminal.writeln('\r\n\x1b[33m[连接已关闭]\x1b[0m');
          // 尝试重连
          scheduleReconnect(terminal);
        };

        ws.onerror = () => {
          setConnectionError('连接失败');
          terminal.writeln('\r\n\x1b[31m[连接错误]\x1b[0m');
        };

        // 发送用户输入到 WebSocket
        terminal.onData((data) => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'input', data }));
          }
        });

        // 发送 resize 事件
        terminal.onResize(({ rows, cols }) => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'resize', rows, cols }));
          }
        });

        wsRef.current.ws = ws;
        setIsLoading(false);
      } catch (err) {
        setConnectionError('连接初始化失败');
        scheduleReconnect(terminal);
      }
    };

    // 重连调度函数
    const scheduleReconnect = (term: Terminal) => {
      const { reconnectAttempts, reconnectTimeout } = wsRef.current;

      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }

      if (reconnectAttempts >= WS_RECONNECT_CONFIG.maxAttempts) {
        term.writeln(`\x1b[31m[连接]\x1b[0m 最大重连次数已到达，请刷新页面`);
        return;
      }

      // 计算延迟时间（指数退避）
      const delay = Math.min(
        WS_RECONNECT_CONFIG.baseDelay * Math.pow(WS_RECONNECT_CONFIG.backoffMultiplier, reconnectAttempts),
        WS_RECONNECT_CONFIG.maxDelay
      );

      wsRef.current.reconnectAttempts += 1;

      term.writeln(`\x1b[33m[重连]\x1b[0m ${delay/1000}秒后尝试第${wsRef.current.reconnectAttempts}次重连...`);

      wsRef.current.reconnectTimeout = setTimeout(() => {
        connectWebSocket();
      }, delay);
    };

    connectWebSocket();

    // 监听 resize
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // Ignore fit errors during rapid resize
      }
    });
    resizeObserver.observe(terminalRef.current);

    return () => {
      resizeObserver.disconnect();
      if (wsRef.current.reconnectTimeout) {
        clearTimeout(wsRef.current.reconnectTimeout);
      }
      if (wsRef.current.ws) {
        wsRef.current.ws.close();
      }
      terminal.dispose();
    };
  }, []);

  // 外部调整大小时重新 fit
  useEffect(() => {
    const handleResize = () => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        // Ignore fit errors during rapid resize
      }
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return (
    <div className="h-full flex flex-col bg-[#1e1e1e] rounded-md overflow-hidden">
      {/* 终端标题栏 */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#252526] border-b border-[#3c3c3c]">
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400 truncate">{title}</span>
          <span className={cn(
            'w-2 h-2 rounded-full transition-colors',
            isConnected ? 'bg-green-500' : connectionError ? 'bg-red-500' : 'bg-yellow-500 animate-pulse'
          )} />
          {connectionError && (
            <span className="text-xs text-red-400 truncate">{connectionError}</span>
          )}
        </div>
        {onClose && (
          <button
            onClick={onClose}
            className="p-0.5 rounded hover:bg-[#3c3c3c] text-gray-400 hover:text-gray-200"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      {/* 终端内容 */}
      <div className="flex-1 relative">
        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-[#1e1e1e]">
            <div className="text-gray-400 text-sm">连接中...</div>
          </div>
        )}
        <div ref={terminalRef} className="h-full w-full" />
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
              : 'hover:bg-accent text-muted-foreground'
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
              activeTabId === tab.id
                ? 'bg-primary text-primary-foreground'
                : 'hover:bg-accent'
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
export function TerminalGrid() {
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

  return (
    <div
      className={cn(
        'flex flex-col bg-card border rounded-lg overflow-hidden',
        isFullscreen ? 'fixed inset-4 z-50' : 'h-full'
      )}
    >
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

      {/* 终端网格 */}
      <div className="flex-1 overflow-hidden">
        <Allotment>
          {Array.from({ length: totalPanes }).map((_, index) => {
            const terminal = activeTerminals[index];
            return (
              <Allotment.Pane
                key={terminal?.id || `empty-${index}`}
                minSize={100}
              >
                {terminal ? (
                  <TerminalPane
                    terminalId={terminal.id}
                    title={terminal.title}
                  />
                ) : (
                  <div className="h-full flex items-center justify-center bg-[#1e1e1e]">
                    <div className="text-gray-500 text-sm">
                      暂无终端 - 点击 + 添加
                    </div>
                  </div>
                )}
              </Allotment.Pane>
            );
          })}
        </Allotment>
      </div>
    </div>
  );
}
