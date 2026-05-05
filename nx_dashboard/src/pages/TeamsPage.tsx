import { useEffect, useState } from 'react';
import { useTeamStore, Team, Role } from '@/stores/teamStore';
import { useWorkspaceStore, onWorkspaceChange } from '@/stores/workspaceStore';
import { useTeamsQuery } from '@/hooks/useReactQuery';
import {
  Plus,
  Trash2,
  X,
  Users,
  Clock,
  Sparkles,
  MessageCircle,
  Bot,
  Zap,
  Loader2,
  Eye,
  Radio,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { TeamDetailPanel } from '@/components/team/TeamDetailPanel';
import { RoleEditor } from '@/components/team/RoleEditor';
import { ConversationView } from '@/components/team/ConversationView';
import { TelegramConfigPanel } from '@/components/team/TelegramConfigPanel';
import { AddExistingRoleModal } from '@/components/team/AddExistingRoleModal';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';

export function TeamsPage() {
  const {
    getTeam,
    deleteTeam,
    setCurrentTeam,
    roles,
    fetchRoles,
    teamMonitorMode,
    setTeamMonitorMode,
  } = useTeamStore();
  const { currentWorkspace } = useWorkspaceStore();
  const [selectedTeam, setSelectedTeam] = useState<Team | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showRoleEditor, setShowRoleEditor] = useState(false);
  const [editingRole, setEditingRole] = useState<Role | null>(null);
  const [showConversation, setShowConversation] = useState(false);
  const [showTelegramConfig, setShowTelegramConfig] = useState(false);
  const [showAddExistingRole, setShowAddExistingRole] = useState(false);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  // Use React Query for fetching
  const { teams, loading, refetch } = useTeamsQuery();

  // Listen for workspace changes
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

  const [, setRolesLoading] = useState(false);

  const handleCardClick = async (team: Team, e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button')) return;

    setRolesLoading(true);
    const fullTeam = await getTeam(team.id);
    if (fullTeam) {
      setCurrentTeam(fullTeam);
      setSelectedTeam(fullTeam);
      // Fetch roles and wait for them to load before showing panel
      await fetchRoles(fullTeam.id);
    }
    setRolesLoading(false);
  };

  const handleEditRole = (role: Role) => {
    setEditingRole(role);
    setShowRoleEditor(true);
  };

  const handleCreateRole = () => {
    setEditingRole(null);
    setShowRoleEditor(true);
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
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              智能体团队
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">管理您的多智能体协作团队</p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建团队
        </button>
      </div>

      {/* Guide Card */}
      <div className="bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5 border border-indigo-500/20 rounded-2xl p-5">
        <div className="flex items-start gap-4">
          <div className="p-2.5 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
            <Sparkles className="w-5 h-5 text-white" />
          </div>
          <div>
            <p className="font-semibold mb-2">团队操作指南</p>
            <ul className="grid grid-cols-2 gap-2 text-sm text-muted-foreground">
              <li className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-indigo-500" />
                点击卡片查看团队详情
              </li>
              <li className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-purple-500" />
                管理团队中的角色
              </li>
              <li className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-pink-500" />
                分配技能给角色
              </li>
              <li className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-blue-500" />
                配置 Telegram 通知
              </li>
            </ul>
          </div>
        </div>
      </div>

      {/* Teams Grid */}
      {teams.length === 0 ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <Users className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无团队</h3>
          <p className="text-muted-foreground mb-4">创建您的第一个智能体团队</p>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            <Plus className="w-4 h-4" />
            新建团队
          </button>
        </div>
      ) : (
        <div className="grid gap-4 stagger-children">
          {teams.map((team) => (
            <div
              key={team.id}
              className={cn(
                'bg-card rounded-2xl border border-border/50 p-5',
                'hover:shadow-lg hover:shadow-primary/5 hover:border-primary/20',
                'transition-all duration-200 cursor-pointer group',
                'hover:-translate-y-0.5',
              )}
              onClick={(e) => handleCardClick(team, e)}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-2">
                    <h3 className="font-semibold text-lg group-hover:text-indigo-600 transition-colors">
                      {team.name}
                    </h3>
                    <span className="px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-600 text-xs font-medium">
                      团队
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground mb-3">
                    {team.description || '无描述'}
                  </p>
                  <div className="flex items-center gap-3 flex-wrap">
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-gradient-to-r from-indigo-500/10 to-purple-500/10 text-xs font-medium text-indigo-600 border border-indigo-500/20">
                      <Bot className="w-3 h-3" />
                      {roles[team.id]?.length || 0} 角色
                    </span>
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
                    onClick={async () => {
                      const fullTeam = await getTeam(team.id);
                      if (fullTeam) {
                        setSelectedTeam(fullTeam);
                        setShowConversation(true);
                      }
                    }}
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
                    onClick={async () => {
                      const fullTeam = await getTeam(team.id);
                      if (fullTeam) {
                        setSelectedTeam(fullTeam);
                        setShowTelegramConfig(true);
                      }
                    }}
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      'bg-gradient-to-r from-blue-500 to-cyan-500',
                      'text-white shadow-lg shadow-blue-500/25',
                      'hover:shadow-blue-500/40 hover:-translate-y-0.5',
                    )}
                    title="Telegram"
                  >
                    <Zap className="w-4 h-4" />
                  </button>
                  <button
                    onClick={(e) => handleCardClick(team, e)}
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      'bg-gradient-to-r from-indigo-500 to-purple-500',
                      'text-white shadow-lg shadow-indigo-500/25',
                      'hover:shadow-indigo-500/40 hover:-translate-y-0.5',
                    )}
                    title="详情"
                  >
                    <Eye className="w-4 h-4" />
                  </button>
                  <button
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
                  {/* 监控模式开关 */}
                  <button
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
          ))}
        </div>
      )}

      {/* Team Detail Panel */}
      {selectedTeam && !showConversation && !showTelegramConfig && (
        <TeamDetailPanel
          team={selectedTeam}
          roles={roles[selectedTeam.id] || []}
          onClose={() => setSelectedTeam(null)}
          onEditRole={handleEditRole}
          onCreateRole={() => handleCreateRole()}
          onDeleteRole={(roleId) => {
            const role = roles[selectedTeam.id]?.find((r) => r.id === roleId);
            showConfirm(
              '移除角色',
              `确定要从团队中移除角色「${role?.name ?? roleId}」吗？`,
              () => useTeamStore.getState().unassignRoleFromTeam(roleId, selectedTeam.id),
              'warning',
            );
          }}
          onOpenConversation={() => setShowConversation(true)}
          onOpenTelegramConfig={() => setShowTelegramConfig(true)}
          onAddExistingRole={() => setShowAddExistingRole(true)}
          onTeamUpdated={() => {
            // Refresh selected team with updated data
            getTeam(selectedTeam.id).then((updated) => {
              if (updated) {
                setSelectedTeam(updated);
              }
            });
          }}
          projectId={currentWorkspace?.id}
        />
      )}

      {/* Create Team Modal */}
      {showCreateModal && (
        <CreateTeamModal onClose={() => setShowCreateModal(false)} onCreate={handleCreateTeam} />
      )}

      {/* Role Editor Modal */}
      {showRoleEditor && selectedTeam && (
        <RoleEditor
          role={editingRole}
          teamId={selectedTeam.id}
          onClose={() => {
            setShowRoleEditor(false);
            setEditingRole(null);
          }}
          onSave={() => {
            setShowRoleEditor(false);
            setEditingRole(null);
            if (selectedTeam) {
              fetchRoles(selectedTeam.id);
            }
          }}
        />
      )}

      {/* Add Existing Role Modal */}
      {showAddExistingRole && selectedTeam && (
        <AddExistingRoleModal
          teamId={selectedTeam.id}
          onClose={() => setShowAddExistingRole(false)}
          onAdded={() => {
            if (selectedTeam) {
              fetchRoles(selectedTeam.id);
            }
          }}
        />
      )}

      {/* Conversation View */}
      {showConversation && selectedTeam && (
        <ConversationView teamId={selectedTeam.id} onClose={() => setShowConversation(false)} />
      )}

      {/* Telegram Config Panel */}
      {showTelegramConfig && selectedTeam && (
        <TelegramConfigPanel
          teamId={selectedTeam.id}
          onClose={() => setShowTelegramConfig(false)}
        />
      )}

      {/* Confirm Modal */}
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

// Create Team Modal Component
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
            <label className="block text-sm font-medium mb-2">团队名称</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="输入团队名称"
              className="input-field"
              autoFocus
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">描述 (可选)</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="输入团队描述"
              className="input-field resize-none"
              rows={3}
            />
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button type="submit" disabled={creating || !name.trim()} className="btn-primary">
              {creating ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  创建中...
                </>
              ) : (
                <>
                  <Plus className="w-4 h-4" />
                  创建
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
