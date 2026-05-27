import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ChevronDown, Cpu, Factory, Users } from 'lucide-react';
import { api, type ClaudeCliConfigResponse } from '@/api/client';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useTeamStore } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { useExecutionStore } from '@/stores/executionStore';
import { cn } from '@/lib/utils';

export function FactoryGlobalBar() {
  const navigate = useNavigate();
  const { currentWorkspace } = useWorkspaceStore();
  const { teams, currentTeam, fetchTeams, setCurrentTeam } = useTeamStore();
  const { projects, currentProject, fetchProjects, setCurrentProject } = useProjectStore();
  const runningCount = useExecutionStore((s) => s.executions.filter((e) => e.status === 'running').length);
  const [cliConfig, setCliConfig] = useState<ClaudeCliConfigResponse | null>(null);

  useEffect(() => {
    fetchTeams();
    fetchProjects();
  }, [fetchTeams, fetchProjects]);

  useEffect(() => {
    let cancelled = false;
    api.getClaudeCliConfig().then((cfg) => {
      if (!cancelled) setCliConfig(cfg);
    }).catch(() => {});
    return () => { cancelled = true; };
  }, []);

  const cliBound = cliConfig?.source !== 'none' && !!cliConfig?.path;

  return (
    <div className="flex items-center gap-2 sm:gap-4 min-w-0 flex-wrap">
      <div className="flex items-center gap-2 text-sm font-medium text-foreground">
        <Factory className="w-4 h-4 text-primary shrink-0" />
        <span className="hidden sm:inline">工厂台</span>
      </div>

      <Link
        to="/settings/ai"
        title={cliBound ? cliConfig?.path ?? 'Claude CLI' : '未绑定 Claude CLI — 点击配置'}
        className={cn(
          'hidden sm:flex items-center gap-1.5 text-[10px] px-2 py-0.5 rounded-full border',
          cliBound
            ? 'border-emerald-500/30 text-emerald-700 dark:text-emerald-400'
            : 'border-amber-500/40 text-amber-700 dark:text-amber-400',
        )}
      >
        <Cpu className="w-3 h-3" />
        <span className={cn('w-1.5 h-1.5 rounded-full', cliBound ? 'bg-emerald-500' : 'bg-amber-500')} />
        {cliBound ? 'CLI' : 'CLI 未绑定'}
      </Link>

      <select
        className="text-xs sm:text-sm bg-muted/50 border border-border rounded-md px-2 py-1 max-w-[140px] truncate"
        value={currentProject?.id ?? ''}
        onChange={(e) => {
          const p = projects.find((x) => x.id === e.target.value) ?? null;
          setCurrentProject(p);
        }}
        title="项目"
      >
        <option value="">项目…</option>
        {projects.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>

      <div className="flex items-center gap-1">
        <Users className="w-3.5 h-3.5 text-muted-foreground" />
        <select
          className="text-xs sm:text-sm bg-muted/50 border border-border rounded-md px-2 py-1 max-w-[140px] truncate"
          value={currentTeam?.id ?? ''}
          onChange={(e) => {
            const t = teams.find((x) => x.id === e.target.value) ?? null;
            setCurrentTeam(t);
          }}
          title="团队"
        >
          <option value="">团队…</option>
          {teams.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </div>

      {currentWorkspace?.root_path && (
        <span
          className="hidden lg:inline text-xs text-muted-foreground truncate max-w-[200px]"
          title={currentWorkspace.root_path}
        >
          {currentWorkspace.root_path}
        </span>
      )}

      <button
        type="button"
        onClick={() => navigate('/factory?tab=runs')}
        className={cn(
          'ml-auto flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full border transition-colors',
          runningCount > 0
            ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
            : 'border-border text-muted-foreground hover:bg-accent',
        )}
      >
        <span className={cn('w-2 h-2 rounded-full', runningCount > 0 ? 'bg-emerald-500 animate-pulse' : 'bg-muted-foreground/40')} />
        {runningCount > 0 ? `${runningCount} 运行中` : '无运行'}
        <ChevronDown className="w-3 h-3 opacity-50" />
      </button>
    </div>
  );
}
