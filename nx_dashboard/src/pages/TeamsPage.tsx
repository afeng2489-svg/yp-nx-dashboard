import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTeamStore, Team } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { useExecutionStore } from '@/stores/executionStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { useTeamsQuery } from '@/hooks/useReactQuery';
import { showInfo } from '@/lib/toast';
import {
  Plus,
  Trash2,
  X,
  Users,
  Clock,
  MessageCircle,
  Bot,
  Zap,
  Radio,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';
import { TeamTemplatePicker } from '@/components/team/TeamTemplatePicker';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageGuideBanner } from '@/components/ui/PageGuideBanner';
import { PageEmptyState } from '@/components/ui/PageEmptyState';

export function TeamsPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { projects } = useProjectStore();
  const executions = useExecutionStore((s) => s.executions);
  const { deleteTeam, roles, teamMonitorMode, setTeamMonitorMode } = useTeamStore();
  const [showCreateModal, setShowCreateModal] = useState(false);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const { teams, loading, refetch } = useTeamsQuery();

  useEffect(() => {
    const tab = searchParams.get('tab');
    if (tab === 'discuss' && teams.length > 0) {
      const teamId = searchParams.get('team') ?? teams[0]?.id;
      if (teamId) {
        navigate(`/teams/${teamId}?tab=discuss`, { replace: true });
        return;
      }
    }
    if (tab === 'discuss') {
      showInfo('请先选择团队', '从下方卡片进入团队后再打开「讨论」Tab');
      setSearchParams({}, { replace: true });
    }
  }, [searchParams, teams, navigate, setSearchParams]);

  useEffect(() => {
    void useExecutionStore.getState().fetchExecutions();
  }, []);

  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => {
      unsubscribe();
    };
  }, [refetch]);

  const handleCreateTeam = async (teamData: { name: string; description?: string }) => {
    try {
      await useTeamStore.getState().createTeam(teamData);
      refetch();
      setShowCreateModal(false);
    } catch (error) {
      console.error('Failed to create team:', error);
      showError('操作失败', '创建团队失败');
    }
  };

  const openTeam = (teamId: string, tab?: string) => {
    navigate(tab ? `/teams/${teamId}?tab=${tab}` : `/teams/${teamId}`);
  };

  const handleCardClick = (team: Team, e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button')) return;
    openTeam(team.id);
  };

  const handleDeleteTeam = (team: Team) => {
    showConfirm(
      '删除团队',
      `确定删除团队 "${team.name}"？`,
      () => {
        deleteTeam(team.id);
        refetch();
      },
      'danger',
    );
  };

  const activeRunCount = (teamId: string) =>
    executions.filter(
      (e) => e.team_id === teamId && (e.status === 'running' || e.status === 'paused'),
    ).length;

  if (loading) {
    return (
      <div className="page-container">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-32" />
          <div className="h-40 bg-muted rounded-xl" />
          <div className="h-40 bg-muted rounded-xl" />
        </div>
      </div>
    );
  }

  return (
    <div className="page-container space-y-6">
      <PageHeader
        title="智能体团队"
        description="管理您的多智能体协作团队"
        actions={
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            <Plus className="w-4 h-4" />
            新建团队
          </button>
        }
      />

      <PageGuideBanner title="团队操作指南">
        <ul className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm text-muted-foreground">
          <li className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary" />
            点击卡片进入团队详情
          </li>
          <li className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary/70" />
            详情页：成员 / 对话 / 讨论 / Pipeline
          </li>
          <li className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary/50" />
            快捷按钮直达对话或 Telegram 设置
          </li>
          <li className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-muted-foreground" />
            卡片显示绑定项目与活跃 Run
          </li>
        </ul>
      </PageGuideBanner>

      <div className="rounded-2xl border border-border/50 p-5">
        <p className="font-semibold mb-3 text-sm">从模板创建团队</p>
        <TeamTemplatePicker />
      </div>

      {teams.length === 0 ? (
        <PageEmptyState
          icon={Users}
          title="暂无团队"
          description="创建您的第一个智能体团队"
          action={
            <button onClick={() => setShowCreateModal(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              新建团队
            </button>
          }
        />
      ) : (
        <div className="grid gap-4 stagger-children">
          {teams.map((team) => {
            const boundProject = projects.find((p) => p.team_id === team.id);
            const runs = activeRunCount(team.id);
            return (
              <div
                key={team.id}
                className={cn(
                  'bg-card rounded-2xl border border-border/50 p-5',
                  'hover:border-primary/30 hover:shadow-md',
                  'transition-all duration-200 cursor-pointer group',
                )}
                onClick={(e) => handleCardClick(team, e)}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="font-semibold text-lg group-hover:text-primary transition-colors">
                        {team.name}
                      </h3>
                      <span className="px-2 py-0.5 rounded-full bg-primary/10 text-primary text-xs font-medium">
                        团队
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground mb-3">
                      {team.description || '无描述'}
                    </p>
                    <div className="flex items-center gap-3 flex-wrap">
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-muted text-xs font-medium text-muted-foreground border border-border/50">
                        <Bot className="w-3 h-3" />
                        {roles[team.id]?.length || 0} 角色
                      </span>
                      {boundProject && (
                        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground px-2 py-0.5 rounded-md bg-muted/50">
                          项目: {boundProject.name}
                        </span>
                      )}
                      {runs > 0 && (
                        <span className="inline-flex items-center gap-1 text-xs text-blue-600 px-2 py-0.5 rounded-md bg-blue-500/10">
                          {runs} 活跃 Run
                        </span>
                      )}
                      <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                        <Clock className="w-3 h-3" />
                        {team.updated_at
                          ? new Date(team.updated_at).toLocaleString('zh-CN', {
                              month: 'short',
                              day: 'numeric',
                              hour: '2-digit',
                              minute: '2-digit',
                            })
                          : '未知'}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 ml-4" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      onClick={() => openTeam(team.id, 'chat')}
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        'bg-gradient-to-r from-emerald-500 to-green-500',
                        'text-white shadow-lg shadow-emerald-500/25',
                        'hover:shadow-emerald-500/40 hover:-translate-y-0.5',
                      )}
                      title="对话"
                    >
                      <MessageCircle className="w-4 h-4" />
                    </button>
                    <button
                      type="button"
                      onClick={() => openTeam(team.id, 'settings')}
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        'bg-gradient-to-r from-blue-500 to-cyan-500',
                        'text-white shadow-lg shadow-blue-500/25',
                        'hover:shadow-blue-500/40 hover:-translate-y-0.5',
                      )}
                      title="Telegram 设置"
                    >
                      <Zap className="w-4 h-4" />
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDeleteTeam(team)}
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        'hover:bg-red-500/10 text-muted-foreground hover:text-red-500',
                        'hover:-translate-y-0.5',
                      )}
                      title="删除"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        setTeamMonitorMode(team.id, !(teamMonitorMode[team.id] ?? false))
                      }
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        teamMonitorMode[team.id]
                          ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-lg shadow-amber-500/25 hover:shadow-amber-500/40'
                          : 'hover:bg-accent text-muted-foreground hover:text-foreground',
                        'hover:-translate-y-0.5',
                      )}
                      title={
                        teamMonitorMode[team.id]
                          ? '监控模式（点击切换为自动模式）'
                          : '自动模式（点击切换为监控模式）'
                      }
                    >
                      <Radio className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {showCreateModal && (
        <CreateTeamModal onClose={() => setShowCreateModal(false)} onCreate={handleCreateTeam} />
      )}

      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={() => {
          confirmState.onConfirm();
          hideConfirm();
        }}
        onCancel={hideConfirm}
        variant={confirmState.variant || 'danger'}
      />
    </div>
  );
}

interface CreateTeamModalProps {
  onClose: () => void;
  onCreate: (team: { name: string; description?: string }) => Promise<void> | void;
}

function CreateTeamModal({ onClose, onCreate }: CreateTeamModalProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [creating, setCreating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setCreating(true);
    try {
      await onCreate({ name: name.trim(), description: description.trim() || undefined });
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
              <Users className="w-5 h-5 text-white" />
            </div>
            <h2 className="text-lg font-semibold">新建团队</h2>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">团队名称</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-field w-full"
              placeholder="例如：全栈开发组"
              autoFocus
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">描述（可选）</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="input-field w-full resize-none"
              rows={3}
              placeholder="团队职责与协作方式"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button type="submit" disabled={!name.trim() || creating} className="btn-primary">
              {creating ? '创建中…' : '创建'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
