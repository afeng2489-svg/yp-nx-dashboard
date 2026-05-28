import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Cpu, Factory } from 'lucide-react';
import { useClaudeCliReady } from '@/hooks/useClaudeCliReady';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useTeamStore } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { GlobalRunPanel } from '@/components/factory/GlobalRunPanel';
import { GlobalBranchChip } from '@/components/factory/GlobalBranchChip';
import { GlobalCostChip } from '@/components/factory/GlobalCostChip';
import { cn } from '@/lib/utils';

export function FactoryGlobalBar() {
  const { currentWorkspace } = useWorkspaceStore();
  const { teams, currentTeam, fetchTeams, setCurrentTeam } = useTeamStore();
  const { projects, currentProject, fetchProjects, setCurrentProject } = useProjectStore();
  const { ready: cliReady, config: cliConfig } = useClaudeCliReady();
  const [teamManuallyChanged, setTeamManuallyChanged] = useState(false);
  const autoSyncedKeyRef = useRef<string | null>(null);

  useEffect(() => {
    fetchTeams();
    fetchProjects();
  }, [fetchTeams, fetchProjects]);

  /** AF-08 M1：切项目自动切绑定团队（用户未手动改团队时） */
  useEffect(() => {
    if (!currentProject?.team_id || teamManuallyChanged) return;
    const syncKey = `${currentProject.id}:${currentProject.team_id}`;
    if (autoSyncedKeyRef.current === syncKey) return;
    const bound = teams.find((t) => t.id === currentProject.team_id);
    if (bound && bound.id !== currentTeam?.id) {
      setCurrentTeam(bound);
    }
    autoSyncedKeyRef.current = syncKey;
  }, [
    currentProject?.team_id,
    currentProject?.id,
    teams,
    currentTeam?.id,
    teamManuallyChanged,
    setCurrentTeam,
  ]);

  const cliBound = cliReady === true;
  const boundTeamName = currentProject
    ? teams.find((t) => t.id === currentProject.team_id)?.name
    : undefined;

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
        {cliBound ? 'CLI' : cliReady === null ? 'CLI…' : 'CLI 未绑定'}
      </Link>

      <select
        className="text-xs sm:text-sm bg-muted/50 border border-border rounded-md px-2 py-1 max-w-[140px] truncate"
        value={currentProject?.id ?? ''}
        onChange={(e) => {
          const p = projects.find((x) => x.id === e.target.value) ?? null;
          setTeamManuallyChanged(false);
          autoSyncedKeyRef.current = null;
          setCurrentProject(p);
          if (p?.team_id) {
            const t = teams.find((x) => x.id === p.team_id) ?? null;
            if (t) setCurrentTeam(t);
          }
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
        <select
          className="text-xs sm:text-sm bg-muted/50 border border-border rounded-md px-2 py-1 max-w-[140px] truncate"
          value={currentTeam?.id ?? ''}
          onChange={(e) => {
            setTeamManuallyChanged(true);
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
        {currentProject && boundTeamName && currentTeam?.id === currentProject.team_id && (
          <span className="hidden md:inline text-[10px] text-emerald-600 dark:text-emerald-400" title="项目已绑定团队">
            已绑定
          </span>
        )}
      </div>

      <GlobalBranchChip />
      <GlobalCostChip />

      {currentWorkspace?.root_path && (
        <span
          className="hidden lg:inline text-xs text-muted-foreground truncate max-w-[200px]"
          title={currentWorkspace.root_path}
        >
          {currentWorkspace.root_path}
        </span>
      )}

      <GlobalRunPanel />
    </div>
  );
}
