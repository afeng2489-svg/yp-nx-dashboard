import { useState } from 'react';
import {
  X,
  Edit,
  Trash2,
  Plus,
  Bot,
  MessageCircle,
  Zap,
  Clock,
  Users,
  UserPlus,
  GitBranch,
  Settings,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Team, Role } from '@/stores/teamStore';
import { RoleCard } from './RoleCard';
import PipelineView from './PipelineView';
import FeatureFlagPanel from './FeatureFlagPanel';

type TabKey = 'roles' | 'pipeline' | 'settings';

interface TeamDetailPanelProps {
  team: Team;
  roles: Role[];
  onClose: () => void;
  onEditRole: (role: Role) => void;
  onCreateRole: () => void;
  onDeleteRole: (roleId: string) => void;
  onOpenConversation: () => void;
  onOpenTelegramConfig: () => void;
  onAddExistingRole: () => void;
  onTeamUpdated?: () => void;
  projectId?: string;
}

export function TeamDetailPanel({
  team,
  roles,
  onClose,
  onEditRole,
  onCreateRole,
  onDeleteRole,
  onOpenConversation,
  onOpenTelegramConfig,
  onAddExistingRole,
  onTeamUpdated,
  projectId,
}: TeamDetailPanelProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(team.name);
  const [editDescription, setEditDescription] = useState(team.description || '');
  const [activeTab, setActiveTab] = useState<TabKey>('roles');

  const handleSave = async () => {
    await useTeamStore.getState().updateTeam(team.id, {
      name: editName,
      description: editDescription,
    });
    setIsEditing(false);
    onTeamUpdated?.();
  };

  const handleCancel = () => {
    setEditName(team.name);
    setEditDescription(team.description || '');
    setIsEditing(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div
        className="absolute inset-0 bg-gradient-to-r from-black/20 to-black/60 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Users className="w-5 h-5 text-indigo-500" />
            团队详情
          </h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-border/50">
          {[
            { key: 'roles' as TabKey, label: '角色', icon: Bot },
            { key: 'pipeline' as TabKey, label: 'Pipeline', icon: GitBranch },
            { key: 'settings' as TabKey, label: '设置', icon: Settings },
          ].map(({ key, label, icon: Icon }) => (
            <button
              key={key}
              onClick={() => setActiveTab(key)}
              className={cn(
                'flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium border-b-2 transition-colors',
                activeTab === key
                  ? 'border-indigo-500 text-indigo-600'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              <Icon className="w-4 h-4" />
              {label}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {activeTab === 'roles' && (
            <>
              {/* Name & Description */}
              <div className="space-y-2">
                {isEditing ? (
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="input-field text-xl font-bold"
                  />
                ) : (
                  <h3 className="text-2xl font-bold bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent">
                    {team.name}
                  </h3>
                )}
              </div>

              <div className="space-y-2">
                <h4 className="text-sm font-medium text-muted-foreground">描述</h4>
                {isEditing ? (
                  <textarea
                    value={editDescription}
                    onChange={(e) => setEditDescription(e.target.value)}
                    className="input-field resize-none"
                    rows={3}
                  />
                ) : (
                  <p className="text-sm text-muted-foreground/80 leading-relaxed">
                    {team.description || '暂无描述'}
                  </p>
                )}
              </div>

              {/* Edit Controls */}
              <div className="flex gap-2">
                {isEditing ? (
                  <>
                    <button onClick={handleSave} className="btn-primary text-sm py-1.5">
                      保存
                    </button>
                    <button onClick={handleCancel} className="btn-secondary text-sm py-1.5">
                      取消
                    </button>
                  </>
                ) : (
                  <button onClick={() => setIsEditing(true)} className="btn-ghost text-sm gap-1.5">
                    <Edit className="w-3.5 h-3.5" /> 编辑
                  </button>
                )}
              </div>

              {/* Quick Actions */}
              <div className="grid grid-cols-2 gap-3">
                <button
                  onClick={onOpenConversation}
                  className={cn(
                    'flex items-center gap-3 p-4 rounded-xl border border-border/50',
                    'hover:border-emerald-500/50 hover:bg-emerald-500/5',
                    'transition-all duration-200',
                  )}
                >
                  <div className="p-2 rounded-lg bg-gradient-to-br from-emerald-500 to-green-500">
                    <MessageCircle className="w-4 h-4 text-white" />
                  </div>
                  <div className="text-left">
                    <p className="font-medium text-sm">对话</p>
                    <p className="text-xs text-muted-foreground">与团队聊天</p>
                  </div>
                </button>
                <button
                  onClick={onOpenTelegramConfig}
                  className={cn(
                    'flex items-center gap-3 p-4 rounded-xl border border-border/50',
                    'hover:border-blue-500/50 hover:bg-blue-500/5',
                    'transition-all duration-200',
                  )}
                >
                  <div className="p-2 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-500">
                    <Zap className="w-4 h-4 text-white" />
                  </div>
                  <div className="text-left">
                    <p className="font-medium text-sm">Telegram</p>
                    <p className="text-xs text-muted-foreground">配置通知</p>
                  </div>
                </button>
              </div>

              {/* Roles Section */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                    <Bot className="w-4 h-4" />
                    角色 ({roles.length})
                  </h4>
                  <button onClick={onCreateRole} className="btn-ghost text-sm gap-1.5">
                    <Plus className="w-3.5 h-3.5" /> 新建
                  </button>
                  <button onClick={onAddExistingRole} className="btn-ghost text-sm gap-1.5">
                    <UserPlus className="w-3.5 h-3.5" /> 添加已有
                  </button>
                </div>
                <div className="space-y-3">
                  {roles.length === 0 ? (
                    <div className="text-center py-8 text-muted-foreground">
                      <Bot className="w-8 h-8 mx-auto mb-2 opacity-50" />
                      <p className="text-sm">暂无角色</p>
                      <button onClick={onCreateRole} className="btn-primary text-sm mt-2">
                        <Plus className="w-4 h-4" /> 添加角色
                      </button>
                    </div>
                  ) : (
                    roles.map((role) => (
                      <RoleCard
                        key={role.id}
                        role={role}
                        onEdit={() => onEditRole(role)}
                        onDelete={() => onDeleteRole(role.id)}
                      />
                    ))
                  )}
                </div>
              </div>

              {/* Timestamps */}
              <div className="pt-4 border-t border-border/50">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Clock className="w-3.5 h-3.5" />
                  <span>
                    创建:{' '}
                    {team.created_at ? new Date(team.created_at).toLocaleString('zh-CN') : '未知'}
                  </span>
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                  <Clock className="w-3.5 h-3.5" />
                  <span>
                    更新:{' '}
                    {team.updated_at ? new Date(team.updated_at).toLocaleString('zh-CN') : '未知'}
                  </span>
                </div>
              </div>
            </>
          )}

          {activeTab === 'pipeline' &&
            (projectId ? (
              <PipelineView projectId={projectId} />
            ) : (
              <div className="text-sm text-muted-foreground text-center py-8">
                请先打开一个项目工作区以查看 Pipeline
              </div>
            ))}

          {activeTab === 'settings' && <FeatureFlagPanel />}
        </div>
      </div>
    </div>
  );
}

// Import useTeamStore for updateTeam
import { useTeamStore } from '@/stores/teamStore';
