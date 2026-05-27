import { useEffect, useState } from 'react';
import { API_BASE_URL } from '@/api/constants';
import { useExecutionStore } from '@/stores/executionStore';

/** 底栏：CLI / WS / permissions 状态 */
export function FactoryStatusBar() {
  const [permMode, setPermMode] = useState<string>('…');
  const wsConnected = useExecutionStore((s) => {
    for (const status of s.wsConnectionStatus.values()) {
      if (status === 'connected') return true;
    }
    return false;
  });
  const runningCount = useExecutionStore((s) => s.executions.filter((e) => e.status === 'running').length);

  useEffect(() => {
    fetch(`${API_BASE_URL}/api/v1/ai/security/permissions-mode`)
      .then((r) => r.json())
      .then((d) => {
        const mode = d.data?.mode ?? d.mode;
        if (mode) setPermMode(String(mode));
      })
      .catch(() => setPermMode('strict'));
  }, []);

  return (
    <footer className="h-7 px-4 flex items-center justify-between text-[10px] sm:text-xs text-muted-foreground border-t border-border/50 bg-card/30 shrink-0">
      <span>Claude CLI · permissions: {permMode}</span>
      <span>
        WS: {wsConnected ? '已连接' : runningCount > 0 ? '连接中…' : '空闲'}
      </span>
    </footer>
  );
}
