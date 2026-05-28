import { useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { Terminal } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { api } from '@/api/client';
import { useExecutionStore, type WsConnectionStatus } from '@/stores/executionStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';

function aggregateWsLabel(
  statuses: WsConnectionStatus[],
  runningOrPaused: number,
): { text: string; tone: 'ok' | 'warn' | 'muted' } {
  if (statuses.length === 0) {
    return runningOrPaused > 0
      ? { text: '轮询中', tone: 'warn' }
      : { text: '空闲', tone: 'muted' };
  }
  if (statuses.some((s) => s === 'polling')) {
    return { text: '轮询中', tone: 'warn' };
  }
  if (statuses.some((s) => s === 'reconnecting' || s === 'connecting')) {
    return { text: '重连中', tone: 'warn' };
  }
  if (statuses.some((s) => s === 'connected')) {
    return { text: '已连接', tone: 'ok' };
  }
  return { text: '断开', tone: 'warn' };
}

function shortPath(path: string): string {
  if (path.length <= 36) return path;
  const parts = path.split(/[/\\]/);
  return parts.length > 2 ? `…/${parts.slice(-2).join('/')}` : path;
}

/** 底栏：CLI / WS / 工作区 / 当前 Run 阶段 */
export function FactoryStatusBar() {
  const location = useLocation();
  const openDrawer = useFactoryDrawerStore((s) => s.open);
  const onFactory = location.pathname.startsWith('/factory');
  const [permMode, setPermMode] = useState<string>('…');
  const [cliModel, setCliModel] = useState<string>('…');
  const wsStatuses = useExecutionStore((s) => [...s.wsConnectionStatus.values()]);
  const executions = useExecutionStore((s) => s.executions);
  const runningOrPaused = useExecutionStore(
    (s) => s.executions.filter((e) => e.status === 'running' || e.status === 'paused').length,
  );
  const selectedExecutionId = useContextPanelStore((s) => s.selectedExecutionId);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);

  const ws = useMemo(
    () => aggregateWsLabel(wsStatuses, runningOrPaused),
    [wsStatuses, runningOrPaused],
  );

  const activeStage = useMemo(() => {
    if (selectedExecutionId) {
      const ex = executions.find((e) => e.id === selectedExecutionId);
      if (ex?.current_stage) return ex.current_stage;
    }
    const running = executions.find((e) => e.status === 'running');
    return running?.current_stage;
  }, [executions, selectedExecutionId]);

  useEffect(() => {
    fetch(`${API_BASE_URL}/api/v1/ai/security/permissions-mode`)
      .then((r) => r.json())
      .then((d) => {
        const mode = d.data?.mode ?? d.mode;
        if (mode) setPermMode(String(mode));
      })
      .catch(() => setPermMode('strict'));
    api
      .getClaudeCliModel()
      .then((d) => setCliModel(d.primary_model || d.sonnet_model))
      .catch(() => setCliModel('unknown'));
  }, []);

  return (
    <footer className="h-7 px-4 flex items-center justify-between gap-2 text-[10px] sm:text-xs text-muted-foreground border-t border-border/50 bg-card/30 shrink-0 min-w-0">
      <span className="truncate shrink min-w-0">
        CLI · {cliModel} · permissions: {permMode}
        {currentWorkspace?.root_path && (
          <span className="hidden lg:inline text-muted-foreground/80">
            {' '}
            · {shortPath(currentWorkspace.root_path)}
          </span>
        )}
      </span>
      <span className="flex items-center gap-2 shrink-0">
        {onFactory && (
          <button
            type="button"
            onClick={() => openDrawer('terminal')}
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-accent/50 text-foreground/80"
            title="打开工具抽屉"
          >
            <Terminal className="w-3 h-3" />
            工具
          </button>
        )}
        {activeStage && (
          <span className="hidden sm:inline truncate max-w-[120px]" title={activeStage}>
            Stage: {activeStage}
          </span>
        )}
        <span
          className={
            ws.tone === 'ok'
              ? 'text-emerald-600 dark:text-emerald-400'
              : ws.tone === 'warn'
                ? 'text-amber-600 dark:text-amber-400'
                : undefined
          }
        >
          WS: {ws.text}
        </span>
      </span>
    </footer>
  );
}
