import { useState, useEffect } from 'react';
import { X, Bot, Search, Loader2, Users } from 'lucide-react';
import { Role } from '@/stores/teamStore';
import { useTeamStore } from '@/stores/teamStore';
import { showSuccess, showError } from '@/lib/toast';

interface AddExistingRoleModalProps {
  teamId: string;
  onClose: () => void;
  onAdded: () => void;
}

export function AddExistingRoleModal({ teamId, onClose, onAdded }: AddExistingRoleModalProps) {
  const { listAllRoles, assignRoleToTeam, roles } = useTeamStore();
  const [allRoles, setAllRoles] = useState<Role[]>([]);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    const fetchAllRoles = async () => {
      setLoading(true);
      try {
        const roles = await listAllRoles();
        setAllRoles(roles);
      } catch (error) {
        console.error('Failed to fetch roles:', error);
      } finally {
        setLoading(false);
      }
    };
    fetchAllRoles();
  }, [listAllRoles]);

  // Filter out roles that are already in this team
  const currentTeamRoleIds = new Set((roles[teamId] || []).map(r => r.id));
  const availableRoles = allRoles.filter(role => {
    const notInTeam = !currentTeamRoleIds.has(role.id);
    const matchesSearch = searchTerm === '' ||
      role.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      role.description?.toLowerCase().includes(searchTerm.toLowerCase());
    return notInTeam && matchesSearch;
  });

  const handleAddRole = async (roleId: string) => {
    if (!teamId) {
      showError('错误', '团队 ID 不存在');
      return;
    }
    setAdding(roleId);
    try {
      await assignRoleToTeam(roleId, teamId);
      showSuccess('添加成功', '角色已添加到团队');
      // Don't call onAdded() - assignRoleToTeam already does optimistic update
      // Calling fetchRoles would add the role a second time (duplicate!)
      onClose();
    } catch (error) {
      const message = error instanceof Error ? error.message : '添加失败';
      showError('添加失败', message);
      console.error('Failed to add role:', error);
    } finally {
      setAdding(null);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg max-h-[80vh] bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
              <Bot className="w-5 h-5 text-white" />
            </div>
            <h2 className="text-lg font-semibold">添加已有角色</h2>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Search */}
        <div className="px-6 py-3 border-b border-border/50">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="搜索角色..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </div>
        </div>

        {/* Role List */}
        <div className="flex-1 overflow-y-auto p-4">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
            </div>
          ) : availableRoles.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Users className="w-8 h-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">没有可添加的角色</p>
              <p className="text-xs mt-1">所有角色都已在当前团队中</p>
            </div>
          ) : (
            <div className="space-y-2">
              {availableRoles.map(role => (
                <div
                  key={role.id}
                  className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center">
                      <Bot className="w-4 h-4 text-primary" />
                    </div>
                    <div>
                      <p className="font-medium text-sm">{role.name}</p>
                      <p className="text-xs text-muted-foreground">{role.description || '无描述'}</p>
                    </div>
                  </div>
                  <button
                    onClick={() => handleAddRole(role.id)}
                    disabled={adding !== null}
                    className="px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
                  >
                    {adding === role.id ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      '添加'
                    )}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}