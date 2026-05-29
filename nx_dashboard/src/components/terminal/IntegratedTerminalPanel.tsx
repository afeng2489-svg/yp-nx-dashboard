import { useEffect, useRef, useState } from 'react';
import { ChevronDown, Plus, RefreshCw, X } from 'lucide-react';
import { TerminalGrid } from '@/components/terminal/TerminalGrid';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import { useTerminalStore } from '@/stores/terminalStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { cn } from '@/lib/utils';

/** Cursor 风格底部集成终端：Tab + 拖拽分割（由外层 Allotment 控制） */
export function IntegratedTerminalPanel({ visible }: { visible: boolean }) {
  const sessionEpoch = useFactoryDrawerStore((s) => s.sessionEpoch);
  const resetTerminalSession = useFactoryDrawerStore((s) => s.resetTerminalSession);
  const hideIntegrated = useFactoryDrawerStore((s) => s.hideIntegrated);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const { tabs, activeTabId, setActiveTab, addTab, removeTab } = useTerminalStore();
  const bodyRef = useRef<HTMLDivElement>(null);
  const [bodyHeight, setBodyHeight] = useState<number>();

  useEffect(() => {
    const el = bodyRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      setBodyHeight(entry.contentRect.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const handleAddTab = () => {
    const newTabId = addTab({ title: `终端 ${tabs.length + 1}` });
    useTerminalStore.getState().addTerminal({
      tabId: newTabId,
      title: `终端 ${tabs.length + 1}`,
    });
    setActiveTab(newTabId);
  };

  return (
    <div className="integrated-terminal-panel h-full min-h-0 flex flex-col bg-card border-t border-border">
      <div className="flex items-center h-9 shrink-0 border-b border-border bg-muted/20 px-1 gap-1">
        <div className="flex items-center gap-0.5 overflow-x-auto flex-1 min-w-0">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={cn(
                'group inline-flex items-center gap-1.5 h-8 px-2.5 text-xs shrink-0 border-b-2 transition-colors',
                activeTabId === tab.id
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-accent/40',
              )}
            >
              <span className="truncate max-w-[140px]">{tab.title}</span>
              {tabs.length > 1 && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeTab(tab.id);
                  }}
                  className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-accent"
                >
                  <X className="w-3 h-3" />
                </button>
              )}
            </button>
          ))}
          <button
            type="button"
            onClick={handleAddTab}
            className="p-1.5 rounded hover:bg-accent text-muted-foreground shrink-0"
            title="新建终端"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>
        <button
          type="button"
          onClick={resetTerminalSession}
          className="p-1.5 rounded hover:bg-accent text-muted-foreground shrink-0"
          title="新会话"
        >
          <RefreshCw className="w-3.5 h-3.5" />
        </button>
        <button
          type="button"
          onClick={hideIntegrated}
          className="p-1.5 rounded hover:bg-accent text-muted-foreground shrink-0"
          title="隐藏终端面板 (⌃`)"
        >
          <ChevronDown className="w-4 h-4" />
        </button>
      </div>

      <div ref={bodyRef} className="flex-1 min-h-0 overflow-hidden">
        <TerminalGrid
          key={sessionEpoch}
          initialCwd={currentWorkspace?.root_path}
          integrated
          visible={visible}
          panelHeight={bodyHeight}
        />
      </div>
    </div>
  );
}
