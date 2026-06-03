import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  FolderOpen,
  Loader2,
  Plus,
  Trash2,
  ExternalLink,
  Play,
  Clock,
} from 'lucide-react';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useTeamStore } from '@/stores/teamStore';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { workspaceTeamId, workspacesForTeam } from '@/lib/workspaceTeam';
import PipelineView from '@/components/team/PipelineView';
import { TeamEvolutionSection } from '@/pages/GroupChatPage/TeamEvolutionSection';
import { cn } from '@/lib/utils';
import { showError } from '@/lib/toast';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageEmptyState } from '@/components/ui/PageEmptyState';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';

/** 工作区管理 — 工作区即项目，Run 绑定在工作区上执行 */
export function WorkspacesPage() {
  const navigate = useNavigate();
  const { workspaces, loading, fetchWorkspaces, bindWorkspaceTeam, deleteWorkspace, selectWorkspace } =
    useWorkspaceStore();
  const { teams, fetchTeams } = useTeamStore();
  const executions = useExecutionStore((s) => s.executions);
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();
  const [bindingId, setBindingId] = useState<string | null>(null);

  useEffect(() => {
    void fetchWorkspaces();
    void fetchTeams();
    void fetchExecutions();
  }, [fetchWorkspaces, fetchTeams, fetchExecutions]);

  const runsByWorkspace = useMemo(() => {
    const map = new Map<string, Execution[]>();
    for (const e of executions) {
      const wid = e.project_id;
      if (!wid) continue;
      const list = map.get(wid) ?? [];
      list.push(e);
      map.set(wid, list);
    }
    return map;
  }, [executions]);

  const handleBindTeam = async (workspaceId: string, teamId: string) => {
    setBindingId(workspaceId);
    try {
      await bindWorkspaceTeam(workspaceId, teamId === '__none__' ? null : teamId);
    } catch {
      showError('绑定失败', '无法更新工作区团队');
    } finally {
      setBindingId(null);
    }
  };

  const handleDelete = (id: string, name: string) => {
    showConfirm('删除工作区', `确定从列表移除「${name}」？（不会删除本地文件夹）`, () => {
      void deleteWorkspace(id);
    }, 'danger');
  };

  const openInFactory = (workspaceId: string) => {
    const ws = workspaces.find((w) => w.id === workspaceId);
    if (ws) selectWorkspace(ws);
    navigate('/factory');
  };

  return (
    <div className="page-container max-w-4xl mx-auto">
      <PageHeader
        title="工作区"
        description="工作区 = 项目（文件夹 + 名称）。产线 Run 在某一工作区上执行；可绑定团队以便协作。"
        className="mb-6"
      />

      {loading && workspaces.length === 0 ? (
        <div className="flex justify-center py-16">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
        </div>
      ) : workspaces.length === 0 ? (
        <PageEmptyState
          icon={FolderOpen}
          title="还没有工作区"
          description="在工厂台新建项目，或打开本地文件夹，会自动注册为工作区"
          action={
            <Link to="/factory" className="btn btn-primary">
              前往工厂台
            </Link>
          }
        />
      ) : (
        <div className="space-y-3">
          {workspaces.map((ws) => {
            const teamId = workspaceTeamId(ws);
            const teamName = teams.find((t) => t.id === teamId)?.name;
            const runs = runsByWorkspace.get(ws.id) ?? [];
            const activeRuns = runs.filter(
              (r) => r.status === 'running' || r.status === 'paused',
            ).length;

            return (
              <div
                key={ws.id}
                className="rounded-xl border border-border/60 bg-card p-4 shadow-sm"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <FolderOpen className="h-4 w-4 shrink-0 text-muted-foreground" />
                      <h3 className="truncate font-semibold">{ws.name}</h3>
                      {activeRuns > 0 && (
                        <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-[11px] text-blue-600">
                          {activeRuns} Run 进行中
                        </span>
                      )}
                    </div>
                    {ws.description && (
                      <p className="mt-1 text-sm text-muted-foreground">{ws.description}</p>
                    )}
                    {ws.root_path && (
                      <p className="mt-1 truncate font-mono text-xs text-muted-foreground" title={ws.root_path}>
                        {ws.root_path}
                      </p>
                    )}
                    <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                      <span className="flex items-center gap-1">
                        <Play className="h-3 w-3" />
                        {runs.length} 次 Run
                      </span>
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {new Date(ws.updated_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2 shrink-0">
                    <Select
                      value={teamId ?? '__none__'}
                      onValueChange={(v) => void handleBindTeam(ws.id, v)}
                      disabled={bindingId === ws.id}
                    >
                      <SelectTrigger className="h-9 w-[140px] text-xs">
                        <SelectValue placeholder="绑定团队" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="__none__">不绑定团队</SelectItem>
                        {teams.map((t) => (
                          <SelectItem key={t.id} value={t.id}>
                            {t.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <button
                      type="button"
                      onClick={() => openInFactory(ws.id)}
                      className="btn btn-ghost h-9 border border-border px-3 text-sm"
                    >
                      <ExternalLink className="h-4 w-4" />
                      工厂
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDelete(ws.id, ws.name)}
                      className="btn-icon h-9 w-9 text-muted-foreground hover:text-destructive"
                      title="移除"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>
                </div>

                {teamName && (
                  <p className="mt-2 text-xs text-muted-foreground">
                    团队：<span className="font-medium text-foreground">{teamName}</span>
                  </p>
                )}
              </div>
            );
          })}
        </div>
      )}

      <div className="mt-6 rounded-xl border border-dashed border-border/60 p-4 text-sm text-muted-foreground">
        <p className="font-medium text-foreground">模型说明</p>
        <ul className="mt-2 list-inside list-disc space-y-1 text-xs leading-relaxed">
          <li>工作区 = 唯一「项目」概念（本地文件夹 + 名称）</li>
          <li>Run = 在某工作区上的一次产线执行（见运营 → Runs）</li>
          <li>团队 = 工作区 metadata 中的 team_id，或 Run 上的 team_id</li>
        </ul>
      </div>

      {confirmState.isOpen && (
        <ConfirmModal
          isOpen={confirmState.isOpen}
          title={confirmState.title}
          message={confirmState.message}
          onConfirm={() => {
            confirmState.onConfirm?.();
            hideConfirm();
          }}
          onCancel={hideConfirm}
          variant={confirmState.variant || 'danger'}
        />
      )}
    </div>
  );
}

/** 团队详情内嵌：绑定工作区 + 团队 Run 列表 */
export function TeamWorkspaceRunsPanel({ teamId }: { teamId: string }) {
  const navigate = useNavigate();
  const { workspaces, fetchWorkspaces, bindWorkspaceTeam, selectWorkspace } = useWorkspaceStore();
  const { teams } = useTeamStore();
  const executions = useExecutionStore((s) => s.executions);
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);

  useEffect(() => {
    void fetchWorkspaces();
    void fetchExecutions();
  }, [fetchWorkspaces, fetchExecutions]);

  const bound = workspacesForTeam(workspaces, teamId);
  const unbound = workspaces.filter((w) => !workspaceTeamId(w));
  const teamRuns = useMemo(
    () =>
      executions
        .filter((e) => e.team_id === teamId)
        .sort(
          (a, b) =>
            new Date(b.started_at ?? 0).getTime() - new Date(a.started_at ?? 0).getTime(),
        )
        .slice(0, 12),
    [executions, teamId],
  );

  const teamName = teams.find((t) => t.id === teamId)?.name ?? '团队';

  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">绑定的工作区</h2>
          {unbound.length > 0 && (
            <Select
              onValueChange={(wsId) => void bindWorkspaceTeam(wsId, teamId)}
            >
              <SelectTrigger className="h-9 w-auto max-w-[200px] text-xs">
                <Plus className="mr-1 h-3.5 w-3.5" />
                <SelectValue placeholder="绑定工作区" />
              </SelectTrigger>
              <SelectContent>
                {unbound.map((w) => (
                  <SelectItem key={w.id} value={w.id}>
                    {w.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>

        {bound.length === 0 ? (
          <div className="rounded-xl border border-border/60 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
            <p>「{teamName}」尚未绑定工作区</p>
            <p className="mt-1 text-xs">
              从上方选择已有工作区，或在{' '}
              <Link to="/settings/projects" className="text-primary hover:underline">
                设置 → 工作区
              </Link>{' '}
              管理
            </p>
          </div>
        ) : (
          <ul className="space-y-2">
            {bound.map((ws) => (
              <li
                key={ws.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-border/60 bg-card px-4 py-3"
              >
                <div className="min-w-0">
                  <p className="truncate font-medium text-sm">{ws.name}</p>
                  {ws.root_path && (
                    <p className="truncate font-mono text-[11px] text-muted-foreground">
                      {ws.root_path}
                    </p>
                  )}
                </div>
                <button
                  type="button"
                  className="btn btn-primary h-8 shrink-0 px-3 text-xs"
                  onClick={() => {
                    selectWorkspace(ws);
                    navigate('/factory');
                  }}
                >
                  在工厂打开
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      {bound.length > 0 && (
        <section className="space-y-3">
          <h2 className="text-sm font-semibold">团队进化 / Pipeline</h2>
          <p className="text-xs text-muted-foreground">
            进度与 Pipeline 按工作区 scope 存储，创建时会自动绑定当前团队的 team_id。
          </p>
          <div className="space-y-4">
            {bound.map((ws) => (
              <div
                key={ws.id}
                className="rounded-xl border border-border/60 bg-card overflow-hidden"
              >
                <div className="border-b border-border/60 px-4 py-2.5">
                  <p className="text-sm font-medium truncate">{ws.name}</p>
                </div>
                <TeamEvolutionSection workspaceId={ws.id} forceVisible />
                <div className="border-t border-border/60">
                  <PipelineView workspaceId={ws.id} teamId={teamId} />
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold">团队 Run</h2>
          <Link to="/ops?tab=runs" className="text-xs text-primary hover:underline">
            查看全部
          </Link>
        </div>
        {teamRuns.length === 0 ? (
          <p className="text-sm text-muted-foreground py-4 text-center">暂无 Run 记录</p>
        ) : (
          <ul className="space-y-2">
            {teamRuns.map((run) => (
              <li key={run.id}>
                <Link
                  to={`/ops?tab=runs&execution=${run.id}`}
                  className={cn(
                    'flex items-center justify-between gap-3 rounded-lg border border-border/60 px-4 py-2.5',
                    'hover:border-primary/30 hover:bg-accent/30 transition-colors',
                  )}
                >
                  <span className="truncate text-sm font-medium">{run.workflow_id}</span>
                  <span
                    className={cn(
                      'shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium',
                      run.status === 'running' && 'bg-blue-500/10 text-blue-600',
                      run.status === 'completed' && 'bg-green-500/10 text-green-600',
                      run.status === 'failed' && 'bg-destructive/10 text-destructive',
                      (run.status === 'paused' || run.status === 'pending') &&
                        'bg-amber-500/10 text-amber-700',
                    )}
                  >
                    {run.status}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
