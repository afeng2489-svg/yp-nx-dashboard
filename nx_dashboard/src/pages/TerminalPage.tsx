import { useState, useEffect, useCallback, useRef } from 'react';
import { Allotment } from 'allotment';
import { TerminalGrid } from '@/components/terminal';
import { ExecutionMonitor } from '@/components/execution';
import { FileSidebar } from '@/components/explorer';
import {
  useTerminalStore,
  initWindowSync,
  requestSync,
  type TerminalState,
} from '@/stores/terminalStore';
import { useUIStore } from '@/stores/uiStore';
import {
  Maximize2,
  Minimize2,
  PanelLeftClose,
  PanelLeft,
  ExternalLink,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';

type ViewMode = 'split' | 'terminal' | 'monitor';

// 窗口间同步的 Channel 名称
const SYNC_CHANNEL_NAME = 'nexusflow-terminal-sync';

// 同步消息类型
interface SyncMessage {
  type:
    | 'ping'
    | 'pong'
    | 'new_window_opened'
    | 'sync_request'
    | 'sync_response'
    | 'tab_updated'
    | 'active_tab_changed'
    | 'layout_changed'
    | 'fullscreen_changed'
    | 'terminal_output';
  windowId?: string;
  payload?: unknown;
}

// 窗口管理器 Hook
function useWindowSync() {
  const [otherWindows, setOtherWindows] = useState<Set<string>>(new Set());
  const [windowId] = useState(() => Math.random().toString(36).substring(2, 10));
  const [isSyncing, setIsSyncing] = useState(false);
  const channelRef = useRef<BroadcastChannel | null>(null);
  const store = useTerminalStore();
  const otherWindowsRef = useRef(otherWindows);
  otherWindowsRef.current = otherWindows;

  // 处理同步状态变化
  const handleSyncStateChange = useCallback(
    (state: Partial<TerminalState>) => {
      // 只更新来自其他窗口的状态
      setIsSyncing(true);
      store.syncState(state);
      setTimeout(() => setIsSyncing(false), 100);
    },
    [store],
  );

  // 处理终端输出（来自其他窗口，不重新广播）
  const handleOutputReceive = useCallback(
    (terminalId: string, output: string) => {
      store.receiveLog(terminalId, output);
    },
    [store],
  );

  useEffect(() => {
    // 初始化同步系统
    initWindowSync(windowId, handleSyncStateChange, handleOutputReceive);

    // 创建 BroadcastChannel 用于窗口间通信
    const channel = new BroadcastChannel(SYNC_CHANNEL_NAME);
    channelRef.current = channel;

    // 发送 ping 探索其他窗口
    channel.postMessage({ type: 'ping', windowId } as SyncMessage);

    // 监听其他窗口
    channel.onmessage = (event: MessageEvent<SyncMessage>) => {
      const msg = event.data;
      if (msg.windowId === windowId) return;

      switch (msg.type) {
        case 'ping':
          // 回复 pong
          channel.postMessage({ type: 'pong', windowId } as SyncMessage);
          // 添加新窗口
          setOtherWindows((prev) => new Set([...prev, msg.windowId!]));
          break;
        case 'pong':
          // 添加已知窗口
          setOtherWindows((prev) => new Set([...prev, msg.windowId!]));
          break;
        case 'new_window_opened':
          setOtherWindows((prev) => new Set([...prev, msg.windowId!]));
          break;
        case 'sync_response':
          // 收到完整状态，同步
          if (msg.payload) {
            handleSyncStateChange(msg.payload as Partial<TerminalState>);
          }
          break;
      }
    };

    // 定期 ping 保持同步
    const interval = setInterval(() => {
      channel.postMessage({ type: 'ping', windowId } as SyncMessage);
    }, 5000);

    // 新窗口打开后请求同步
    const handleNewWindow = () => {
      setTimeout(() => {
        requestSync();
      }, 500);
    };
    window.addEventListener('storage', handleNewWindow);

    return () => {
      channel.close();
      clearInterval(interval);
      window.removeEventListener('storage', handleNewWindow);
    };
  }, [windowId, handleSyncStateChange, handleOutputReceive]);

  const openNewWindow = useCallback(() => {
    // 通知其他窗口
    if (channelRef.current) {
      channelRef.current.postMessage({ type: 'new_window_opened', windowId } as SyncMessage);
    }

    // 打开新窗口
    window.open('/terminal', '_blank', 'width=1200,height=800');
  }, [windowId]);

  return { otherWindows, windowId, openNewWindow, isSyncing };
}

// 手动请求同步
function requestManualSync() {
  requestSync();
}

export function TerminalPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('split');
  const { isFullscreen, setFullscreen } = useTerminalStore();
  const { sidebarOpen, toggleSidebar } = useUIStore();
  const { otherWindows, openNewWindow, isSyncing } = useWindowSync();

  // 全屏模式切换
  const toggleFullscreen = () => {
    setFullscreen(!isFullscreen);
  };

  if (isFullscreen) {
    return (
      <div className="h-screen w-screen bg-background p-4">
        <div className="h-full flex flex-col">
          {/* 全屏工具栏 */}
          <div className="flex items-center justify-between px-4 py-2 bg-card border rounded-t-lg">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">终端 - 全屏模式</span>
            </div>
            <button
              onClick={toggleFullscreen}
              className="p-1.5 rounded hover:bg-accent text-muted-foreground"
            >
              <Minimize2 className="w-4 h-4" />
            </button>
          </div>
          <div className="flex-1 bg-card border border-t-0 rounded-b-lg overflow-hidden">
            <TerminalGrid />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between px-4 py-3 bg-card border-b">
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold">终端控制台</h1>

          {/* 视图模式切换 */}
          <div className="flex items-center gap-1 bg-accent rounded-md p-1">
            <button
              onClick={() => setViewMode('split')}
              className={cn(
                'px-3 py-1 rounded text-sm transition-colors',
                viewMode === 'split' ? 'bg-primary text-primary-foreground' : 'hover:bg-accent/80',
              )}
            >
              分屏
            </button>
            <button
              onClick={() => setViewMode('terminal')}
              className={cn(
                'px-3 py-1 rounded text-sm transition-colors',
                viewMode === 'terminal'
                  ? 'bg-primary text-primary-foreground'
                  : 'hover:bg-accent/80',
              )}
            >
              终端
            </button>
            <button
              onClick={() => setViewMode('monitor')}
              className={cn(
                'px-3 py-1 rounded text-sm transition-colors',
                viewMode === 'monitor'
                  ? 'bg-primary text-primary-foreground'
                  : 'hover:bg-accent/80',
              )}
            >
              监控
            </button>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* 同步状态指示器 */}
          {isSyncing && (
            <span className="flex items-center gap-1 text-xs text-blue-500 px-2 py-0.5 bg-blue-500/10 rounded animate-pulse">
              <RefreshCw className="w-3 h-3" />
              同步中...
            </span>
          )}

          {/* 新窗口指示器 */}
          {otherWindows.size > 0 && (
            <span className="flex items-center gap-1 text-xs text-green-500 px-2 py-0.5 bg-green-500/10 rounded">
              <span className="w-2 h-2 rounded-full bg-green-500" />
              {otherWindows.size + 1} 个窗口
            </span>
          )}

          {/* 手动同步按钮 */}
          {otherWindows.size > 0 && (
            <button
              onClick={requestManualSync}
              className="p-1.5 rounded hover:bg-accent text-muted-foreground"
              title="手动同步"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
          )}

          {/* 打开新窗口按钮 */}
          <button
            onClick={openNewWindow}
            className="p-1.5 rounded hover:bg-accent text-muted-foreground"
            title="在新窗口中打开"
          >
            <ExternalLink className="w-4 h-4" />
          </button>

          <button
            onClick={toggleSidebar}
            className="p-1.5 rounded hover:bg-accent text-muted-foreground"
            title={sidebarOpen ? '隐藏侧边栏' : '显示侧边栏'}
          >
            {sidebarOpen ? (
              <PanelLeftClose className="w-4 h-4" />
            ) : (
              <PanelLeft className="w-4 h-4" />
            )}
          </button>
          <button
            onClick={toggleFullscreen}
            className="p-1.5 rounded hover:bg-accent text-muted-foreground"
            title="全屏"
          >
            <Maximize2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* 主内容区域 */}
      <div className="flex-1 overflow-hidden">
        {viewMode === 'split' && (
          <Allotment>
            {/* 左侧: 文件浏览器 */}
            <Allotment.Pane minSize={200} preferredSize={250}>
              <div className="h-full p-2">
                <FileSidebar />
              </div>
            </Allotment.Pane>

            {/* 中间: 终端网格 */}
            <Allotment.Pane minSize={300}>
              <div className="h-full p-2">
                <TerminalGrid />
              </div>
            </Allotment.Pane>

            {/* 右侧: 执行监控 */}
            <Allotment.Pane minSize={280} preferredSize={320}>
              <div className="h-full p-2">
                <ExecutionMonitor />
              </div>
            </Allotment.Pane>
          </Allotment>
        )}

        {viewMode === 'terminal' && (
          <div className="h-full p-2">
            <TerminalGrid />
          </div>
        )}

        {viewMode === 'monitor' && (
          <div className="h-full p-2">
            <ExecutionMonitor />
          </div>
        )}
      </div>
    </div>
  );
}
