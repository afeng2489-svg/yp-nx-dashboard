import { useEffect, useState } from 'react';
import {
  useProjectStore,
  Project,
  ExecuteProjectResponse,
  ProjectModule,
  UpsertModuleRequest,
} from '@/stores/projectStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore, Workspace } from '@/stores/workspaceStore';
import {
  Plus,
  Trash2,
  Play,
  X,
  Loader2,
  FolderOpen,
  Clock,
  CheckCircle,
  XCircle,
  AlertCircle,
  Folder,
  Terminal,
  ChevronDown,
  ChevronRight,
  Layers,
  Circle,
  Loader2 as Spinner,
  Ban,
  Edit3,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showError } from '@/lib/toast';
import { ClaudeStreamPanel } from '@/components/terminal/ClaudeStreamPanel';

// Extended project type that includes local path projects (workspaces)
interface DisplayProject {
  id: string;
  name: string;
  description: string;
  path?: string; // Only for local/workspace projects
  team_id?: string;
  type: 'execution' | 'local';
  status?: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
  created_at: string;
  updated_at: string;
}

export function ProjectsPage() {
  const {
    projects,
    loading,
    executing,
    error,
    projectModules,
    fetchProjects,
    createProject,
    deleteProject,
    executeProject,
    executionResult,
    clearExecutionResult,
    fetchProjectModules,
    upsertProjectModule,
    deleteProjectModule,
  } = useProjectStore();
  const { teams, fetchTeams } = useTeamStore();
  const { workspaces, fetchWorkspaces } = useWorkspaceStore();
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showExecuteModal, setShowExecuteModal] = useState(false);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [executeTask, setExecuteTask] = useState('');
  const [expandedProjectId, setExpandedProjectId] = useState<string | null>(null);
  const [editingModule, setEditingModule] = useState<{
    projectId: string;
    moduleName: string;
  } | null>(null);
  const [newModuleName, setNewModuleName] = useState('');
  const [newModuleStatus, setNewModuleStatus] = useState<UpsertModuleRequest['status']>('pending');
  const [newModuleSummary, setNewModuleSummary] = useState('');
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  useEffect(() => {
    fetchProjects();
    fetchTeams();
    fetchWorkspaces();
  }, []);

  // Combine projects and workspaces into a unified list
  const executionProjectNames = new Set(projects.map((p) => p.name));
  const displayProjects: DisplayProject[] = [
    // Execution projects
    ...projects.map((p) => ({
      id: p.id,
      name: p.name,
      description: p.description || '',
      team_id: p.team_id,
      type: 'execution' as const,
      status: p.status,
      created_at: p.created_at,
      updated_at: p.updated_at,
    })),
    // Local projects (workspaces with root_path), excluding those with same name as an execution project
    ...workspaces
      .filter((w) => w.root_path && !executionProjectNames.has(w.name))
      .map((w) => ({
        id: w.id,
        name: w.name,
        description: w.description || '',
        path: w.root_path,
        type: 'local' as const,
        created_at: w.created_at,
        updated_at: w.updated_at,
      })),
  ];

  // Sort by updated_at descending
  displayProjects.sort(
    (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
  );

  const handleCreateProject = async (data: {
    name: string;
    description: string;
    team_id: string;
    workspace_id?: string;
  }) => {
    try {
      await createProject(data);
      setShowCreateModal(false);
    } catch (e) {
      console.error('Failed to create project:', e);
      showError('操作失败', '创建项目失败');
    }
  };

  const handleDeleteProject = (project: Project) => {
    showConfirm(
      '删除项目',
      `确定删除项目 "${project.name}"？`,
      () => deleteProject(project.id),
      'danger',
    );
  };

  const handleExecuteProject = async () => {
    if (!selectedProject || !executeTask.trim()) return;
    try {
      await executeProject(selectedProject.id, executeTask);
      setShowExecuteModal(false);
      setExecuteTask('');
    } catch (e) {
      console.error('Failed to execute project:', e);
      showError('操作失败', '执行项目失败');
    }
  };

  const toggleProjectModules = (projectId: string) => {
    if (expandedProjectId === projectId) {
      setExpandedProjectId(null);
    } else {
      setExpandedProjectId(projectId);
      fetchProjectModules(projectId);
    }
  };

  const handleAddModule = async (projectId: string) => {
    if (!newModuleName.trim()) return;
    await upsertProjectModule(projectId, {
      module_name: newModuleName.trim(),
      status: newModuleStatus,
      summary: newModuleSummary,
    });
    setNewModuleName('');
    setNewModuleStatus('pending');
    setNewModuleSummary('');
    setEditingModule(null);
    fetchProjectModules(projectId);
  };

  const handleToggleModuleStatus = async (projectId: string, module: ProjectModule) => {
    const nextStatus: Record<ProjectModule['status'], ProjectModule['status']> = {
      pending: 'in_progress',
      in_progress: 'completed',
      completed: 'failed',
      failed: 'pending',
    };
    await upsertProjectModule(projectId, {
      module_name: module.module_name,
      status: nextStatus[module.status],
    });
    fetchProjectModules(projectId);
  };

  const handleDeleteModule = async (projectId: string, moduleId: string) => {
    await deleteProjectModule(projectId, moduleId);
    fetchProjectModules(projectId);
  };

  const getModuleStatusIcon = (status: ProjectModule['status']) => {
    switch (status) {
      case 'pending':
        return <Circle className="w-3.5 h-3.5 text-gray-400" />;
      case 'in_progress':
        return <Spinner className="w-3.5 h-3.5 text-blue-500" />;
      case 'completed':
        return <CheckCircle className="w-3.5 h-3.5 text-green-500" />;
      case 'failed':
        return <Ban className="w-3.5 h-3.5 text-red-500" />;
    }
  };

  const getModuleStatusBg = (status: ProjectModule['status']) => {
    switch (status) {
      case 'pending':
        return 'bg-gray-500/10 text-gray-600';
      case 'in_progress':
        return 'bg-blue-500/10 text-blue-600';
      case 'completed':
        return 'bg-green-500/10 text-green-600';
      case 'failed':
        return 'bg-red-500/10 text-red-600';
    }
  };

  const getModuleStatusLabel = (status: ProjectModule['status']) => {
    switch (status) {
      case 'pending':
        return '待做';
      case 'in_progress':
        return '进行中';
      case 'completed':
        return '已完成';
      case 'failed':
        return '失败';
    }
  };

  const getStatusBadge = (status: Project['status']) => {
    switch (status) {
      case 'pending':
        return (
          <span className="px-2 py-0.5 rounded-full bg-yellow-500/10 text-yellow-600 text-xs font-medium">
            待处理
          </span>
        );
      case 'in_progress':
        return (
          <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 text-xs font-medium">
            进行中
          </span>
        );
      case 'completed':
        return (
          <span className="px-2 py-0.5 rounded-full bg-green-500/10 text-green-600 text-xs font-medium">
            已完成
          </span>
        );
      case 'failed':
        return (
          <span className="px-2 py-0.5 rounded-full bg-red-500/10 text-red-600 text-xs font-medium">
            失败
          </span>
        );
      case 'cancelled':
        return (
          <span className="px-2 py-0.5 rounded-full bg-gray-500/10 text-gray-600 text-xs font-medium">
            已取消
          </span>
        );
    }
  };

  if (loading && projects.length === 0) {
    return (
      <div className="page-container">
        <div className="flex items-center justify-center h-64">
          <Loader2 className="w-8 h-8 animate-spin text-primary" />
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
            <span className="bg-gradient-to-r from-emerald-600 via-teal-600 to-cyan-600 bg-clip-text text-transparent">
              项目管理
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">使用团队智能体执行开发任务</p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建项目
        </button>
      </div>

      {/* Projects Grid */}
      {displayProjects.length === 0 ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-emerald-500/10 to-cyan-500/10 flex items-center justify-center">
            <FolderOpen className="w-10 h-10 text-emerald-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无项目</h3>
          <p className="text-muted-foreground mb-4">创建您的第一个项目开始团队协作</p>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            <Plus className="w-4 h-4" />
            新建项目
          </button>
        </div>
      ) : (
        <div className="grid gap-4 stagger-children">
          {displayProjects.map((project) => (
            <div
              key={project.id}
              className={cn(
                'bg-card rounded-2xl border border-border/50 p-5',
                'hover:shadow-lg hover:shadow-primary/5 hover:border-primary/20',
                'transition-all duration-200 group',
              )}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-2">
                    {/* Project type badge */}
                    <span
                      className={cn(
                        'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium',
                        project.type === 'local'
                          ? 'bg-blue-500/10 text-blue-600'
                          : 'bg-emerald-500/10 text-emerald-600',
                      )}
                    >
                      <Folder className="w-3 h-3" />
                      {project.type === 'local' ? '本地' : '执行'}
                    </span>
                    <h3 className="font-semibold text-lg group-hover:text-emerald-600 transition-colors">
                      {project.name}
                    </h3>
                    {project.type === 'execution' &&
                      project.status &&
                      getStatusBadge(project.status)}
                  </div>
                  <p className="text-sm text-muted-foreground mb-3">
                    {project.description || '无描述'}
                  </p>
                  <div className="flex items-center gap-3 flex-wrap">
                    {project.type === 'local' && project.path && (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-blue-500/10 text-xs font-medium text-blue-600">
                        <FolderOpen className="w-3 h-3" />
                        {project.path.length > 40
                          ? project.path.slice(0, 40) + '...'
                          : project.path}
                      </span>
                    )}
                    {project.type === 'execution' && project.team_id && (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-emerald-500/10 text-xs font-medium text-emerald-600">
                        团队:{' '}
                        {teams.find((t) => t.id === project.team_id)?.name ||
                          String(project.team_id).slice(0, 8)}
                      </span>
                    )}
                    <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                      <Clock className="w-3 h-3" />
                      {new Date(project.updated_at).toLocaleString('zh-CN', {
                        month: 'short',
                        day: 'numeric',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-2 ml-4">
                  {project.type === 'execution' && (
                    <button
                      onClick={() => {
                        // Find the original project for execution
                        const originalProject = projects.find((p) => p.id === project.id);
                        if (originalProject) {
                          setSelectedProject(originalProject);
                          setShowExecuteModal(true);
                        }
                      }}
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        'bg-gradient-to-r from-emerald-500 to-teal-500',
                        'text-white shadow-lg shadow-emerald-500/25',
                        'hover:shadow-emerald-500/40 hover:-translate-y-0.5',
                        executing && 'opacity-50 cursor-not-allowed',
                      )}
                      title="执行项目"
                      disabled={executing}
                    >
                      <Play className="w-4 h-4" />
                    </button>
                  )}
                  {project.type === 'execution' && (
                    <button
                      onClick={() => {
                        const originalProject = projects.find((p) => p.id === project.id);
                        if (originalProject) {
                          handleDeleteProject(originalProject);
                        }
                      }}
                      className={cn(
                        'p-2.5 rounded-xl transition-all duration-200',
                        'hover:bg-red-500/10 text-muted-foreground hover:text-red-500',
                        'hover:-translate-y-0.5',
                      )}
                      title="删除"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              {/* Module status section */}
              {project.type === 'local' && (
                <div className="mt-3 pt-3 border-t border-border/30">
                  <button
                    onClick={() => toggleProjectModules(project.id)}
                    className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors w-full"
                  >
                    <Layers className="w-3.5 h-3.5" />
                    <span>模块状态</span>
                    {projectModules[project.id] && (
                      <span className="text-xs">
                        ({projectModules[project.id].filter((m) => m.status === 'completed').length}
                        /{projectModules[project.id].length})
                      </span>
                    )}
                    {expandedProjectId === project.id ? (
                      <ChevronDown className="w-3.5 h-3.5 ml-auto" />
                    ) : (
                      <ChevronRight className="w-3.5 h-3.5 ml-auto" />
                    )}
                  </button>

                  {expandedProjectId === project.id && (
                    <div className="mt-3 space-y-2">
                      {projectModules[project.id]?.length ? (
                        <>
                          {/* Progress bar */}
                          {(() => {
                            const mods = projectModules[project.id];
                            const completed = mods.filter((m) => m.status === 'completed').length;
                            const pct = Math.round((completed / mods.length) * 100);
                            return (
                              <div className="flex items-center gap-2 mb-2">
                                <div className="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-gradient-to-r from-emerald-500 to-teal-500 rounded-full transition-all duration-300"
                                    style={{ width: `${pct}%` }}
                                  />
                                </div>
                                <span className="text-xs text-muted-foreground">{pct}%</span>
                              </div>
                            );
                          })()}

                          {projectModules[project.id].map((mod) => (
                            <div
                              key={mod.id}
                              className="flex items-center gap-2 p-2 rounded-lg bg-accent/30 group/mod"
                            >
                              <button
                                onClick={() => handleToggleModuleStatus(project.id, mod)}
                                title="切换状态"
                                className="flex-shrink-0 hover:scale-110 transition-transform"
                              >
                                {getModuleStatusIcon(mod.status)}
                              </button>
                              <span className="flex-1 text-sm font-medium truncate">
                                {mod.module_name}
                              </span>
                              <span
                                className={cn(
                                  'px-1.5 py-0.5 rounded text-[10px] font-medium',
                                  getModuleStatusBg(mod.status),
                                )}
                              >
                                {getModuleStatusLabel(mod.status)}
                              </span>
                              {mod.summary && (
                                <span className="text-xs text-muted-foreground truncate max-w-[200px]">
                                  {mod.summary}
                                </span>
                              )}
                              <button
                                onClick={() => handleDeleteModule(project.id, mod.id)}
                                className="opacity-0 group-hover/mod:opacity-100 p-0.5 hover:bg-red-500/10 rounded transition-all"
                                title="删除模块"
                              >
                                <X className="w-3 h-3 text-muted-foreground hover:text-red-500" />
                              </button>
                            </div>
                          ))}
                        </>
                      ) : (
                        <p className="text-xs text-muted-foreground py-1">暂无模块</p>
                      )}

                      {/* Add module form */}
                      {editingModule?.projectId === project.id ? (
                        <div className="flex items-center gap-2 p-2 rounded-lg bg-accent/20">
                          <input
                            type="text"
                            value={newModuleName}
                            onChange={(e) => setNewModuleName(e.target.value)}
                            placeholder="模块名称"
                            className="flex-1 px-2 py-1 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-1 focus:ring-primary/50"
                            autoFocus
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') handleAddModule(project.id);
                              if (e.key === 'Escape') setEditingModule(null);
                            }}
                          />
                          <select
                            value={newModuleStatus}
                            onChange={(e) =>
                              setNewModuleStatus(e.target.value as UpsertModuleRequest['status'])
                            }
                            className="px-2 py-1 text-xs rounded-md border border-border bg-background focus:outline-none"
                          >
                            <option value="pending">待做</option>
                            <option value="in_progress">进行中</option>
                            <option value="completed">已完成</option>
                            <option value="failed">失败</option>
                          </select>
                          <input
                            type="text"
                            value={newModuleSummary}
                            onChange={(e) => setNewModuleSummary(e.target.value)}
                            placeholder="摘要(可选)"
                            className="flex-1 px-2 py-1 text-xs rounded-md border border-border bg-background focus:outline-none focus:ring-1 focus:ring-primary/50"
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') handleAddModule(project.id);
                              if (e.key === 'Escape') setEditingModule(null);
                            }}
                          />
                          <button
                            onClick={() => handleAddModule(project.id)}
                            className="text-xs px-2 py-1 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
                          >
                            添加
                          </button>
                          <button
                            onClick={() => {
                              setEditingModule(null);
                              setNewModuleName('');
                              setNewModuleSummary('');
                            }}
                            className="text-xs px-2 py-1 text-muted-foreground hover:text-foreground"
                          >
                            取消
                          </button>
                        </div>
                      ) : (
                        <button
                          onClick={() =>
                            setEditingModule({ projectId: project.id, moduleName: '' })
                          }
                          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors mt-1"
                        >
                          <Plus className="w-3 h-3" />
                          添加模块
                        </button>
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <CreateProjectModal
          teams={teams}
          workspaces={workspaces}
          onClose={() => setShowCreateModal(false)}
          onCreate={handleCreateProject}
        />
      )}

      {/* Execute Modal */}
      {showExecuteModal && selectedProject && (
        <ExecuteProjectModal
          project={selectedProject}
          task={executeTask}
          onTaskChange={setExecuteTask}
          onClose={() => {
            setShowExecuteModal(false);
            setExecuteTask('');
            clearExecutionResult();
          }}
          onExecute={handleExecuteProject}
          executing={executing}
          result={executionResult}
        />
      )}

      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={confirmState.onConfirm}
        onCancel={hideConfirm}
        variant={confirmState.variant}
      />
    </div>
  );
}

// Create Project Modal Component
function CreateProjectModal({
  teams,
  workspaces,
  onClose,
  onCreate,
}: {
  teams: { id: string; name: string }[];
  workspaces: { id: string; name: string; root_path?: string }[];
  onClose: () => void;
  onCreate: (data: {
    name: string;
    description: string;
    team_id: string;
    workspace_id?: string;
  }) => void;
}) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [teamId, setTeamId] = useState(teams[0]?.id || '');
  const [workspaceId, setWorkspaceId] = useState('');

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <div className="bg-card rounded-2xl shadow-xl w-full max-w-md max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between p-6 border-b border-border">
          <h2 className="text-lg font-semibold">新建项目</h2>
          <button onClick={onClose} className="p-1 hover:bg-accent rounded-lg">
            <X className="w-5 h-5" />
          </button>
        </div>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onCreate({
              name,
              description,
              team_id: teamId,
              workspace_id: workspaceId || undefined,
            });
          }}
          className="p-6 space-y-4"
        >
          <div>
            <label className="block text-sm font-medium mb-2">项目名称</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="输入项目名称"
              className="w-full px-4 py-2 rounded-xl border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">描述</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="输入项目描述（可选）"
              className="w-full px-4 py-2 rounded-xl border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 min-h-[80px]"
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">团队</label>
            <select
              value={teamId}
              onChange={(e) => setTeamId(e.target.value)}
              className="w-full px-4 py-2 rounded-xl border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
              required
            >
              {teams.length === 0 ? (
                <option value="">请先创建团队</option>
              ) : (
                teams.map((team) => (
                  <option key={team.id} value={team.id}>
                    {team.name}
                  </option>
                ))
              )}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">工作区（可选）</label>
            <select
              value={workspaceId}
              onChange={(e) => setWorkspaceId(e.target.value)}
              className="w-full px-4 py-2 rounded-xl border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
            >
              <option value="">不关联工作区</option>
              {workspaces
                .filter((w) => w.root_path)
                .map((workspace) => (
                  <option key={workspace.id} value={workspace.id}>
                    {workspace.name} ({workspace.root_path?.slice(0, 30)}...)
                  </option>
                ))}
            </select>
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button type="submit" className="btn-primary" disabled={!name.trim() || !teamId}>
              创建
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// Execute Project Modal Component
function ExecuteProjectModal({
  project,
  task,
  onTaskChange,
  onClose,
  onExecute,
  executing,
  result,
}: {
  project: Project;
  task: string;
  onTaskChange: (task: string) => void;
  onClose: () => void;
  onExecute: () => void;
  executing: boolean;
  result: ExecuteProjectResponse | null;
}) {
  const [activeTab, setActiveTab] = useState<'standard' | 'cli'>('standard');

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-xl w-full max-w-4xl mx-4 max-h-[85vh] overflow-hidden flex flex-col">
        <div className="flex items-center justify-between p-6 border-b border-border">
          <div>
            <h2 className="text-lg font-semibold">执行项目</h2>
            <p className="text-sm text-muted-foreground">{project.name}</p>
          </div>
          <button onClick={onClose} className="p-1 hover:bg-accent rounded-lg">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Tab navigation */}
        <div className="flex border-b border-border px-6">
          <button
            onClick={() => setActiveTab('standard')}
            className={cn(
              'flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors',
              activeTab === 'standard'
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground',
            )}
          >
            <Play className="w-4 h-4" />
            标准执行
          </button>
          <button
            onClick={() => setActiveTab('cli')}
            className={cn(
              'flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors',
              activeTab === 'cli'
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground',
            )}
          >
            <Terminal className="w-4 h-4" />
            CLI 流式输出
          </button>
        </div>

        {/* Tab content */}
        <div className="flex-1 overflow-y-auto">
          {activeTab === 'standard' ? (
            <div className="p-6 space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">执行任务</label>
                <textarea
                  value={task}
                  onChange={(e) => onTaskChange(e.target.value)}
                  placeholder="描述您想要完成的任务..."
                  className="w-full px-4 py-2 rounded-xl border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 min-h-[120px]"
                  disabled={executing}
                />
              </div>

              {result && (
                <div className="space-y-3">
                  <div
                    className={cn(
                      'flex items-center gap-2 p-3 rounded-xl',
                      result.success
                        ? 'bg-green-500/10 text-green-600'
                        : 'bg-red-500/10 text-red-600',
                    )}
                  >
                    {result.success ? (
                      <CheckCircle className="w-5 h-5" />
                    ) : (
                      <XCircle className="w-5 h-5" />
                    )}
                    <span className="font-medium">{result.success ? '执行成功' : '执行失败'}</span>
                  </div>

                  {result.final_output && (
                    <div>
                      <label className="block text-sm font-medium mb-2">最终输出</label>
                      <div className="p-4 rounded-xl bg-accent/50 text-sm whitespace-pre-wrap max-h-[200px] overflow-y-auto">
                        {result.final_output}
                      </div>
                    </div>
                  )}

                  {result.messages.length > 0 && (
                    <div>
                      <label className="block text-sm font-medium mb-2">
                        执行消息 ({result.messages.length})
                      </label>
                      <div className="space-y-2 max-h-[300px] overflow-y-auto">
                        {result.messages.map((msg, idx) => (
                          <div key={idx} className="p-3 rounded-xl bg-accent/30 text-sm">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="font-medium text-primary">
                                {msg.role_name || 'System'}
                              </span>
                              <span className="text-xs text-muted-foreground">
                                {new Date(msg.created_at).toLocaleTimeString()}
                              </span>
                            </div>
                            <p className="text-xs whitespace-pre-wrap">{msg.content}</p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {result.error && (
                    <div className="p-3 rounded-xl bg-red-500/10 text-red-600 text-sm">
                      <div className="flex items-center gap-2 mb-1">
                        <AlertCircle className="w-4 h-4" />
                        <span className="font-medium">错误</span>
                      </div>
                      {result.error}
                    </div>
                  )}
                </div>
              )}
            </div>
          ) : (
            <div className="h-full min-h-[400px]">
              <ClaudeStreamPanel
                className="h-full rounded-none border-0"
                initialPrompt={task}
                workingDirectory={project.workspace_id}
              />
            </div>
          )}
        </div>

        {/* Footer actions */}
        {activeTab === 'standard' && (
          <div className="flex justify-end gap-3 p-6 border-t border-border">
            <button onClick={onClose} className="btn-secondary">
              关闭
            </button>
            <button
              onClick={onExecute}
              disabled={!task.trim() || executing}
              className="btn-primary"
            >
              {executing ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  执行中...
                </>
              ) : (
                <>
                  <Play className="w-4 h-4" />
                  执行
                </>
              )}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
