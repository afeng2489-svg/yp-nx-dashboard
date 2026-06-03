import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ChevronDown } from 'lucide-react';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { workspaceDisplayName } from '@/lib/workspaceTeam';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { cn } from '@/lib/utils';

function groupLabel(
  execution: Execution,
  teamName: string,
  workspaceName: string,
): string {
  if (workspaceName && teamName) return `${workspaceName} · ${teamName}`;
  if (teamName) return teamName;
  if (workspaceName) return workspaceName;
  return '未绑定工作区/团队';
}

function RunRow({
  execution,
  onSelect,
}: {
  execution: Execution;
  onSelect: () => void;
}) {
  const statusTone =
    execution.status === 'running'
      ? 'text-emerald-600 dark:text-emerald-400'
      : execution.status === 'paused'
        ? 'text-amber-600 dark:text-amber-400'
        : 'text-muted-foreground';

  return (
    <button
      type="button"
      onClick={onSelect}
      className="w-full text-left px-3 py-2 hover:bg-accent/60 rounded-lg transition-colors"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-mono truncate">{execution.id.slice(0, 8)}…</span>
        <span className={cn('text-[10px] shrink-0', statusTone)}>{execution.status}</span>
      </div>
      {execution.current_stage && (
        <p className="text-[10px] text-muted-foreground truncate mt-0.5">
          {execution.current_stage}
        </p>
      )}
    </button>
  );
}

/** AF-08 顶栏 Run 面板 — 按 team/工作区分组 */
export function GlobalRunPanel() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const { executions, fetchExecutions } = useExecutionStore();
  const { teams } = useTeamStore();
  const { workspaces, fetchWorkspaces } = useWorkspaceStore();
  const selectContextExecution = useContextPanelStore((s) => s.selectExecution);

  const active = useMemo(
    () =>
      executions.filter(
        (e) => e.status === 'running' || e.status === 'paused' || e.status === 'pending',
      ),
    [executions],
  );

  const groups = useMemo(() => {
    const map = new Map<string, Execution[]>();
    for (const ex of active) {
      const teamName = teams.find((t) => t.id === ex.team_id)?.name ?? '';
      const workspaceName = workspaceDisplayName(workspaces, ex.project_id) ?? '';
      const key = groupLabel(ex, teamName, workspaceName);
      const list = map.get(key) ?? [];
      list.push(ex);
      map.set(key, list);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b, 'zh-CN'));
  }, [active, teams, workspaces]);

  useEffect(() => {
    void fetchExecutions();
    void fetchWorkspaces();
  }, [fetchExecutions, fetchWorkspaces]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [open]);

  const runningCount = active.filter((e) => e.status === 'running').length;

  const handleSelect = (executionId: string) => {
    selectContextExecution(executionId);
    setOpen(false);
    navigate('/factory?tab=runs');
  };

  return (
    <div className="relative ml-auto" ref={panelRef}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={cn(
          'flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full border transition-colors',
          runningCount > 0
            ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
            : 'border-border text-muted-foreground hover:bg-accent',
        )}
      >
        <span
          className={cn(
            'w-2 h-2 rounded-full',
            runningCount > 0 ? 'bg-emerald-500 animate-pulse' : 'bg-muted-foreground/40',
          )}
        />
        {runningCount > 0 ? `${runningCount} 运行中` : active.length > 0 ? `${active.length} 等待` : '无运行'}
        <ChevronDown className={cn('w-3 h-3 opacity-50 transition-transform', open && 'rotate-180')} />
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-1 z-[100] w-72 max-h-80 overflow-auto rounded-xl border border-border bg-card text-foreground shadow-xl p-2">
          {active.length === 0 ? (
            <p className="text-xs text-muted-foreground px-2 py-4 text-center">暂无活跃 Run</p>
          ) : (
            groups.map(([label, runs]) => (
              <div key={label} className="mb-2 last:mb-0">
                <p className="text-[10px] font-medium text-muted-foreground uppercase px-2 py-1 truncate">
                  {label}
                </p>
                {runs.map((ex) => (
                  <RunRow key={ex.id} execution={ex} onSelect={() => handleSelect(ex.id)} />
                ))}
              </div>
            ))
          )}
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              navigate('/ops?tab=runs');
            }}
            className="w-full mt-1 text-[10px] text-primary hover:underline py-1"
          >
            查看全部历史 →
          </button>
        </div>
      )}
    </div>
  );
}
