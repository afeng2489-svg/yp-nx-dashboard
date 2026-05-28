import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import {
  ArrowLeft,
  Bot,
  GitBranch,
  Loader2,
  MessageCircle,
  MessageSquare,
  Plus,
  Radio,
  Settings,
  UserPlus,
  Users,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, type Role } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { useExecutionStore } from '@/stores/executionStore';
import { RoleCard } from '@/components/team/RoleCard';
import { RoleEditor } from '@/components/team/RoleEditor';
import PipelineView from '@/components/team/PipelineView';
import { TelegramConfigPanel } from '@/components/team/TelegramConfigPanel';
import { AddExistingRoleModal } from '@/components/team/AddExistingRoleModal';
import { TeamChatUnified } from '@/components/team/TeamChatUnified';
import { GroupChatPage } from '@/pages/GroupChatPage';
import { isP5TeamChatUnifiedEnabled } from '@/data/factoryFeatureFlags';
import { showError } from '@/lib/toast';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageTabs } from '@/components/ui/PageTabs';

const TABS = [
  { id: 'members', label: '成员', icon: Users },
  { id: 'chat', label: isP5TeamChatUnifiedEnabled() ? '团队聊天' : '对话', icon: MessageCircle },
  ...(isP5TeamChatUnifiedEnabled()
    ? []
    : [{ id: 'discuss' as const, label: '讨论', icon: MessageSquare }]),
  { id: 'pipelines', label: 'Pipeline', icon: GitBranch },
  { id: 'settings', label: '设置', icon: Settings },
] as const;

type TeamTab = (typeof TABS)[number]['id'];

export function TeamDetailPage() {
  const { teamId = '' } = useParams<{ teamId: string }>();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = (searchParams.get('tab') as TeamTab) || 'members';

  const { getTeam, fetchRoles, roles, setCurrentTeam, deleteRole, teamMonitorMode, setTeamMonitorMode } =
    useTeamStore();
  const { projects } = useProjectStore();
  const executions = useExecutionStore((s) => s.executions);
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);

  const [team, setTeam] = useState<Awaited<ReturnType<typeof getTeam>>>(null);
  const [loading, setLoading] = useState(true);
  const [showRoleEditor, setShowRoleEditor] = useState(false);
  const [editingRole, setEditingRole] = useState<Role | null>(null);
  const [showAddExistingRole, setShowAddExistingRole] = useState(false);

  useEffect(() => {
    void fetchExecutions();
  }, [fetchExecutions]);

  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    setLoading(true);
    (async () => {
      const t = await getTeam(teamId);
      if (cancelled) return;
      if (t) {
        setTeam(t);
        setCurrentTeam(t);
        await fetchRoles(teamId);
      }
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, getTeam, fetchRoles, setCurrentTeam]);

  const teamRoles = roles[teamId] ?? [];
  const boundProject = projects.find((p) => p.team_id === teamId);
  const activeRuns = useMemo(
    () =>
      executions.filter(
        (e) =>
          e.team_id === teamId &&
          (e.status === 'running' || e.status === 'paused'),
      ).length,
    [executions, teamId],
  );

  const setTab = (id: TeamTab) => {
    setSearchParams({ tab: id });
  };

  if (loading) {
    return (
      <div className="page-container flex items-center justify-center min-h-[320px]">
        <Loader2 className="w-8 h-8 animate-spin text-primary" />
      </div>
    );
  }

  if (!team) {
    return (
      <div className="page-container text-center py-16">
        <p className="text-muted-foreground mb-4">团队不存在或已删除</p>
        <Link to="/teams" className="text-primary hover:underline">
          返回团队列表
        </Link>
      </div>
    );
  }

  return (
    <div className="page-container flex flex-col min-h-0 h-full space-y-4 pb-6">
      <PageHeader
        title={team.name}
        description={team.description || '无描述'}
        back={
          <button
            type="button"
            onClick={() => navigate('/teams')}
            className="p-2 rounded-lg hover:bg-accent shrink-0 mt-0.5"
            title="返回"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
        }
        badges={
          <>
            {boundProject && (
              <span className="px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-xs">
                项目: {boundProject.name}
              </span>
            )}
            <span className="px-2 py-0.5 rounded-full bg-muted text-muted-foreground text-xs">
              {teamRoles.length} 角色
            </span>
            {activeRuns > 0 && (
              <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 text-xs">
                {activeRuns} 活跃 Run
              </span>
            )}
          </>
        }
      />

      <PageTabs
        items={TABS.map((t) => ({ id: t.id, label: t.label, icon: t.icon }))}
        value={tab}
        onValueChange={(id) => setTab(id as TeamTab)}
        className="shrink-0"
      />

      <div className="flex-1 min-h-0 overflow-auto">
        {tab === 'members' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                <Bot className="w-4 h-4" />
                角色 ({teamRoles.length})
              </h2>
              <div className="flex gap-2">
                <button
                  type="button"
                  className="btn-ghost text-sm gap-1"
                  onClick={() => {
                    setEditingRole(null);
                    setShowRoleEditor(true);
                  }}
                >
                  <Plus className="w-3.5 h-3.5" /> 新建
                </button>
                <button
                  type="button"
                  className="btn-ghost text-sm gap-1"
                  onClick={() => setShowAddExistingRole(true)}
                >
                  <UserPlus className="w-3.5 h-3.5" /> 添加已有
                </button>
              </div>
            </div>
            {teamRoles.length === 0 ? (
              <p className="text-sm text-muted-foreground py-8 text-center">暂无角色</p>
            ) : (
              <div className="space-y-3">
                {teamRoles.map((role) => (
                  <RoleCard
                    key={role.id}
                    role={role}
                    onEdit={() => {
                      setEditingRole(role);
                      setShowRoleEditor(true);
                    }}
                    onDelete={() => {
                      void deleteRole(role.id).catch(() =>
                        showError('删除失败', '无法删除角色'),
                      );
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        )}

        {tab === 'chat' && <TeamChatUnified teamId={teamId} />}

        {tab === 'discuss' && !isP5TeamChatUnifiedEnabled() && (
          <div className="min-h-[480px]">
            <GroupChatPage embedded teamId={teamId} />
          </div>
        )}

        {tab === 'pipelines' && (
          boundProject ? (
            <PipelineView projectId={boundProject.id} />
          ) : (
            <p className="text-sm text-muted-foreground text-center py-12">
              该团队尚未绑定项目 — 请在设置 → 项目中关联
            </p>
          )
        )}

        {tab === 'settings' && (
          <div className="space-y-4">
            <div className="rounded-xl border border-border/50 p-4 flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium">对话监控模式</p>
                <p className="text-xs text-muted-foreground mt-0.5">
                  开启后 Agent 每步需人工确认后再继续
                </p>
              </div>
              <button
                type="button"
                onClick={() => setTeamMonitorMode(teamId, !(teamMonitorMode[teamId] ?? false))}
                className={cn(
                  'inline-flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors',
                  teamMonitorMode[teamId]
                    ? 'border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300'
                    : 'border-border hover:bg-accent',
                )}
              >
                <Radio className="w-3.5 h-3.5" />
                {teamMonitorMode[teamId] ? '监控模式' : '自动模式'}
              </button>
            </div>
            <div className="rounded-xl border border-border/50 overflow-hidden">
              <TelegramConfigPanel teamId={teamId} onClose={() => navigate('/teams')} />
            </div>
          </div>
        )}
      </div>

      {showRoleEditor && (
        <RoleEditor
          teamId={teamId}
          role={editingRole}
          onClose={() => {
            setShowRoleEditor(false);
            setEditingRole(null);
          }}
          onSave={() => {
            void fetchRoles(teamId);
          }}
        />
      )}

      {showAddExistingRole && (
        <AddExistingRoleModal
          teamId={teamId}
          onClose={() => setShowAddExistingRole(false)}
          onAdded={() => void fetchRoles(teamId)}
        />
      )}
    </div>
  );
}
