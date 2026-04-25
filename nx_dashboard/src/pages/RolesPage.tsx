import { useEffect, useState } from 'react';
import { useTeamStore, Role, Team } from '@/stores/teamStore';
import { Plus, Trash2, Edit, Search, Filter, Bot, Users, Loader2, X, Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';
import { SkillAssigner } from '@/components/team/SkillAssigner';
import { API_BASE_URL } from '@/api/constants';

const API_BASE = API_BASE_URL;

interface RoleWithTeam extends Role {
  teamName?: string;
}

export function RolesPage() {
  const { teams, fetchTeams } = useTeamStore();
  const [allRoles, setAllRoles] = useState<RoleWithTeam[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [filterTeamId, setFilterTeamId] = useState<string>('all');
  const [showRoleModal, setShowRoleModal] = useState(false);
  const [editingRole, setEditingRole] = useState<RoleWithTeam | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newRoleName, setNewRoleName] = useState('');
  const [newRoleDescription, setNewRoleDescription] = useState('');
  const [newRoleTeamId, setNewRoleTeamId] = useState('');
  const [newRoleInstructions, setNewRoleInstructions] = useState('');
  const [confirmDelete, setConfirmDelete] = useState<RoleWithTeam | null>(null);
  const [skillManageRole, setSkillManageRole] = useState<RoleWithTeam | null>(null);
  const [roleSkills, setRoleSkills] = useState<Record<string, string[]>>({});

  // Handle opening skill manager - fetch skills for the role first
  const handleOpenSkillManager = async (role: RoleWithTeam) => {
    try {
      // Fetch skills for this role from API
      const response = await fetch(`${API_BASE}/api/v1/roles/${role.id}/skills`);
      if (response.ok) {
        const skills = await response.json();
        const skillIds = skills.map((s: any) => s.skill_id);
        setRoleSkills(prev => ({ ...prev, [role.id]: skillIds }));
      }
    } catch (error) {
      console.error('Failed to fetch role skills:', error);
    }
    setSkillManageRole(role);
  };

  // Fetch all teams on mount
  useEffect(() => {
    fetchTeams();
  }, [fetchTeams]);

  // Fetch all roles from API
  useEffect(() => {
    let cancelled = false;
    const loadAllRoles = async () => {
      setLoading(true);
      try {
        const response = await fetch(`${API_BASE}/api/v1/roles`);
        if (cancelled) return;
        if (response.ok) {
          const roles: RoleWithTeam[] = await response.json();
          // Deduplicate by id
          const seen = new Set<string>();
          const uniqueRoles = roles.filter(r => {
            if (seen.has(r.id)) return false;
            seen.add(r.id);
            return true;
          });
          // Roles are global now - no team association in the response
          setAllRoles(uniqueRoles);
        }
      } catch (error) {
        if (cancelled) return;
        console.error('Failed to fetch roles:', error);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    loadAllRoles();
    return () => { cancelled = true; };
  }, [teams]);

  const filteredRoles = allRoles.filter(role => {
    const matchesSearch = searchTerm === '' ||
      role.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      role.description?.toLowerCase().includes(searchTerm.toLowerCase());
    return matchesSearch;
  });

  const handleEditRole = (role: RoleWithTeam) => {
    setEditingRole(role);
    setNewRoleName(role.name);
    setNewRoleDescription(role.description || '');
    setNewRoleInstructions(role.instructions || '');
    setShowRoleModal(true);
  };

  const handleSaveRole = async () => {
    if (!editingRole) return;
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${editingRole.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newRoleName,
          description: newRoleDescription,
          system_prompt: newRoleInstructions,
        }),
      });
      if (response.ok) {
        setShowRoleModal(false);
        // Refresh roles
        const rolesRes = await fetch(`${API_BASE}/api/v1/roles`);
        if (rolesRes.ok) {
          const roles: RoleWithTeam[] = await rolesRes.json();
          setAllRoles(roles);
        }
      }
    } catch (error) {
      console.error('Failed to update role:', error);
      showError('操作失败', '更新角色失败');
    }
  };

  const handleDeleteRole = async () => {
    if (!confirmDelete) return;
    try {
      const response = await fetch(`${API_BASE}/api/v1/roles/${confirmDelete.id}`, {
        method: 'DELETE',
      });
      if (response.ok) {
        setAllRoles(prev => prev.filter(r => r.id !== confirmDelete.id));
        setConfirmDelete(null);
      }
    } catch (error) {
      console.error('Failed to delete role:', error);
      showError('操作失败', '删除角色失败');
    }
  };

  const handleCreateRole = async () => {
    if (!newRoleName) return;
    try {
      // Use first team as default if none selected, or allow global creation
      const teamId = newRoleTeamId || teams[0]?.id;
      if (!teamId) {
        alert('请先创建一个团队');
        return;
      }
      const response = await fetch(`${API_BASE}/api/v1/teams/${teamId}/roles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newRoleName,
          description: newRoleDescription || "",
          system_prompt: newRoleInstructions || "",
        }),
      });
      if (response.ok) {
        setShowCreateModal(false);
        setNewRoleName('');
        setNewRoleDescription('');
        setNewRoleInstructions('');
        setNewRoleTeamId('');
        // Refresh roles
        const rolesRes = await fetch(`${API_BASE}/api/v1/roles`);
        if (rolesRes.ok) {
          const roles: RoleWithTeam[] = await rolesRes.json();
          setAllRoles(roles);
        }
      }
    } catch (error) {
      console.error('Failed to create role:', error);
      showError('操作失败', '创建角色失败');
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="p-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Bot className="w-7 h-7" />
            角色管理
          </h1>
          <p className="text-muted-foreground mt-1">
            管理所有团队角色 - 共 {allRoles.length} 个角色
          </p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          <Plus className="w-4 h-4" />
          创建角色
        </button>
      </div>

      {/* Filters */}
      <div className="flex gap-4 mb-6">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            placeholder="搜索角色名称或描述..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-10 pr-4 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
          />
        </div>
        <div className="relative">
          <select
            value={filterTeamId}
            onChange={(e) => setFilterTeamId(e.target.value)}
            className="pl-4 pr-8 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary appearance-none cursor-pointer"
          >
            <option value="all">所有角色</option>
          </select>
        </div>
      </div>

      {/* Roles Grid */}
      {filteredRoles.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Bot className="w-12 h-12 mx-auto mb-4 opacity-50" />
          <p>暂无角色</p>
          <p className="text-sm mt-1">点击上方按钮创建第一个角色</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredRoles.map(role => (
            <div
              key={role.id}
              className="border rounded-lg p-4 bg-card hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
                    <Bot className="w-5 h-5 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold">{role.name}</h3>
                    <p className="text-sm text-muted-foreground">全局角色</p>
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => handleOpenSkillManager(role)}
                    className="p-1.5 hover:bg-accent rounded-md transition-colors"
                    title="分配技能"
                  >
                    <Sparkles className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => handleEditRole(role)}
                    className="p-1.5 hover:bg-accent rounded-md transition-colors"
                    title="编辑"
                  >
                    <Edit className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => setConfirmDelete(role)}
                    className="p-1.5 hover:bg-destructive/10 text-destructive rounded-md transition-colors"
                    title="删除"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>

              {role.description && (
                <p className="text-sm text-muted-foreground mb-3 line-clamp-2">
                  {role.description}
                </p>
              )}

              {role.instructions && (
                <p className="text-xs text-muted-foreground mb-3 line-clamp-2">
                  指令: {role.instructions}
                </p>
              )}

              {role.skills && role.skills.length > 0 && (
                <div className="flex flex-wrap gap-1 mb-3">
                  {role.skills.slice(0, 3).map((skill, idx) => (
                    <span
                      key={idx}
                      className="px-2 py-0.5 text-xs bg-secondary rounded-md"
                    >
                      {skill}
                    </span>
                  ))}
                  {role.skills.length > 3 && (
                    <span className="px-2 py-0.5 text-xs text-muted-foreground">
                      +{role.skills.length - 3}
                    </span>
                  )}
                </div>
              )}

              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>模型: {role.model || '默认'}</span>
                {role.temperature && (
                  <span>温度: {role.temperature}</span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Create Role Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-background rounded-lg p-6 w-full max-w-md shadow-xl">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold">创建角色</h2>
              <button
                onClick={() => setShowCreateModal(false)}
                className="p-1 hover:bg-accent rounded-md"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1">所属团队 (可选)</label>
                <select
                  value={newRoleTeamId}
                  onChange={(e) => setNewRoleTeamId(e.target.value)}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                >
                  <option value="">全局角色 (不关联团队)</option>
                  {teams.map(team => (
                    <option key={team.id} value={team.id}>{team.name}</option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">角色名称 *</label>
                <input
                  type="text"
                  value={newRoleName}
                  onChange={(e) => setNewRoleName(e.target.value)}
                  placeholder="例如：后端开发者"
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">角色描述</label>
                <textarea
                  value={newRoleDescription}
                  onChange={(e) => setNewRoleDescription(e.target.value)}
                  placeholder="描述角色的职责和能力..."
                  rows={3}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">系统指令</label>
                <textarea
                  value={newRoleInstructions}
                  onChange={(e) => setNewRoleInstructions(e.target.value)}
                  placeholder="角色的系统提示词..."
                  rows={4}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
                />
              </div>
            </div>

            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowCreateModal(false)}
                className="flex-1 px-4 py-2 border rounded-lg hover:bg-accent transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleCreateRole}
                disabled={!newRoleName}
                className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                创建
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit Role Modal */}
      {showRoleModal && editingRole && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-background rounded-lg p-6 w-full max-w-md shadow-xl">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold">编辑角色</h2>
              <button
                onClick={() => setShowRoleModal(false)}
                className="p-1 hover:bg-accent rounded-md"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1">角色名称 *</label>
                <input
                  type="text"
                  value={newRoleName}
                  onChange={(e) => setNewRoleName(e.target.value)}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">角色描述</label>
                <textarea
                  value={newRoleDescription}
                  onChange={(e) => setNewRoleDescription(e.target.value)}
                  rows={3}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">系统指令</label>
                <textarea
                  value={newRoleInstructions}
                  onChange={(e) => setNewRoleInstructions(e.target.value)}
                  rows={4}
                  className="w-full px-3 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
                />
              </div>
            </div>

            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowRoleModal(false)}
                className="flex-1 px-4 py-2 border rounded-lg hover:bg-accent transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleSaveRole}
                disabled={!newRoleName}
                className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                保存
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation Modal */}
      <ConfirmModal
        isOpen={!!confirmDelete}
        title="删除角色"
        message={`确定删除角色 "${confirmDelete?.name}"？此操作无法撤销。`}
        confirmText="删除"
        cancelText="取消"
        variant="danger"
        onConfirm={handleDeleteRole}
        onCancel={() => setConfirmDelete(null)}
      />

      {/* Skill Assignment Modal */}
      {skillManageRole && (
        <SkillAssigner
          roleId={skillManageRole.id}
          currentSkills={roleSkills[skillManageRole.id] || skillManageRole.skills || []}
          onClose={() => setSkillManageRole(null)}
          onSkillsChange={(skillIds) => {
            setRoleSkills(prev => ({ ...prev, [skillManageRole.id]: skillIds }));
            // Update the role in the list
            setAllRoles(prev => prev.map(r =>
              r.id === skillManageRole.id ? { ...r, skills: skillIds } : r
            ));
          }}
        />
      )}
    </div>
  );
}