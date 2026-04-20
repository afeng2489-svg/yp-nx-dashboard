import { useEffect, useState } from 'react';
import { useWorkflowsQuery } from '@/hooks/useReactQuery';
import { useNavigate } from 'react-router-dom';
import { useWorkflowStore, Workflow, Agent } from '@/stores/workflowStore';
import { WorkflowTutorialModal } from '@/components/workflow/WorkflowTutorialModal';
import { WorkflowLaunchModal } from '@/components/workflow/WorkflowLaunchModal';
import { useExecutionStore } from '@/stores/executionStore';
import { useSkillStore, type SkillSummary } from '@/stores/skillStore';
import { useWorkspaceStore, onWorkspaceChange } from '@/stores/workspaceStore';
import { Plus, Trash2, Play, Edit, X, Users, GitBranch, Clock, Sparkles, FileText, Wand2, Search, Eye, Palette, Bug, Code2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showSuccess, showError } from '@/lib/toast';
import { API_BASE_URL } from '@/api/constants';

// ── 工作流分类 ──────────────────────────────────────────
type WorkflowCategory = 'all' | 'dev' | 'issue' | 'ui-design';

const CATEGORY_LABELS: Record<WorkflowCategory, string> = {
  all: '全部',
  dev: '开发工具',
  issue: 'Issue 管理',
  'ui-design': 'UI 设计',
};

const CATEGORY_ICONS: Record<WorkflowCategory, React.ReactNode> = {
  all: <GitBranch className="w-3.5 h-3.5" />,
  dev: <Code2 className="w-3.5 h-3.5" />,
  issue: <Bug className="w-3.5 h-3.5" />,
  'ui-design': <Palette className="w-3.5 h-3.5" />,
};

const UI_DESIGN_NAMES = new Set(['style-extract', 'layout-extract', 'animation-extract', 'generate', 'codify-style', 'design-sync']);

function getWorkflowCategory(name: string): WorkflowCategory {
  if (name.startsWith('issue-')) return 'issue';
  if (UI_DESIGN_NAMES.has(name)) return 'ui-design';
  return 'dev';
}


function OperationGuide() {
  const [isVisible, setIsVisible] = useState(true);

  if (!isVisible) return null;

  return (
    <div className="bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5 border border-indigo-500/20 rounded-2xl p-5 relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-r from-indigo-500/5 to-purple-500/5 pointer-events-none" />

      <button
        onClick={() => setIsVisible(false)}
        className="absolute top-3 right-3 p-1.5 hover:bg-indigo-500/10 rounded-lg transition-colors"
      >
        <X className="w-4 h-4 text-muted-foreground" />
      </button>

      <div className="flex items-start gap-4 relative">
        <div className="p-2.5 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
          <Sparkles className="w-5 h-5 text-white" />
        </div>
        <div>
          <p className="font-semibold mb-2">工作流操作指南</p>
          <ul className="grid grid-cols-2 gap-2 text-sm text-muted-foreground">
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-indigo-500" />
              点击卡片查看详情
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-purple-500" />
              点击"编辑"进入可视化编辑器
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-pink-500" />
              点击"执行"运行工作流
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-blue-500" />
              使用模板快速开始
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}

// 从技能创建工作流弹窗组件
interface SkillSelectorModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (skill: SkillSummary) => void;
}

function SkillSelectorModal({ isOpen, onClose, onSelect }: SkillSelectorModalProps) {
  const { skills, categories, loading, fetchSkills, fetchCategories } = useSkillStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);

  useEffect(() => {
    if (isOpen) {
      fetchSkills();
      fetchCategories();
    }
  }, [isOpen, fetchSkills, fetchCategories]);

  const filteredSkills = skills.filter(skill => {
    const matchesSearch = !searchQuery ||
      skill.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      skill.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = !selectedCategory || skill.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const handleGenerate = async (skill: SkillSummary) => {
    setGenerating(true);
    try {
      await onSelect(skill);
      onClose();
    } finally {
      setGenerating(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-2xl max-h-[80vh] bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
              <Wand2 className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">从技能创建工作流</h2>
              <p className="text-sm text-muted-foreground">选择一个技能来生成对应的工作流</p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Search */}
        <div className="px-6 py-4 border-b border-border/50">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="搜索技能..."
              className="w-full pl-10 pr-4 py-2 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
          </div>
        </div>

        {/* Categories */}
        <div className="px-6 py-3 border-b border-border/50 flex gap-2 overflow-x-auto">
          <button
            onClick={() => setSelectedCategory(null)}
            className={cn(
              'px-3 py-1 text-sm rounded-full whitespace-nowrap transition-colors',
              !selectedCategory ? 'bg-primary text-primary-foreground' : 'bg-muted hover:bg-muted/80'
            )}
          >
            全部
          </button>
          {categories.map(cat => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              className={cn(
                'px-3 py-1 text-sm rounded-full whitespace-nowrap transition-colors',
                selectedCategory === cat ? 'bg-primary text-primary-foreground' : 'bg-muted hover:bg-muted/80'
              )}
            >
              {cat.replace(/_/g, ' ')}
            </button>
          ))}
        </div>

        {/* Skills List */}
        <div className="flex-1 overflow-y-auto p-4">
          {loading ? (
            <div className="text-center py-8 text-muted-foreground">加载中...</div>
          ) : filteredSkills.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">没有找到匹配的技能</div>
          ) : (
            <div className="grid gap-3">
              {filteredSkills.map(skill => (
                <button
                  key={skill.id}
                  onClick={() => handleGenerate(skill)}
                  disabled={generating}
                  className={cn(
                    'p-4 rounded-xl border border-border/50 text-left transition-all',
                    'hover:border-primary/50 hover:bg-primary/5',
                    'disabled:opacity-50 disabled:cursor-not-allowed'
                  )}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-semibold">{skill.name}</span>
                        <span className="px-2 py-0.5 text-xs bg-muted rounded">
                          {skill.category.replace(/_/g, ' ')}
                        </span>
                      </div>
                      <p className="text-sm text-muted-foreground line-clamp-2">
                        {skill.description}
                      </p>
                      <div className="flex gap-2 mt-2">
                        {skill.tags.slice(0, 3).map(tag => (
                          <span key={tag} className="px-2 py-0.5 text-xs bg-indigo-500/10 text-indigo-600 rounded">
                            {tag}
                          </span>
                        ))}
                      </div>
                    </div>
                    <Wand2 className="w-5 h-5 text-muted-foreground flex-shrink-0" />
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// 工作流详情面板组件
interface WorkflowDetailPanelProps {
  workflow: Workflow;
  onClose: () => void;
  onEdit: (workflow: Workflow) => void;
}

function WorkflowDetailPanel({ workflow, onClose, onEdit }: WorkflowDetailPanelProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(workflow.name);
  const [editDescription, setEditDescription] = useState(workflow.description || '');
  const { updateWorkflow } = useWorkflowStore();

  const handleSave = async () => {
    await updateWorkflow(workflow.id, {
      name: editName,
      description: editDescription,
    });
    setIsEditing(false);
  };

  const handleCancel = () => {
    setEditName(workflow.name);
    setEditDescription(workflow.description || '');
    setIsEditing(false);
  };

  const agentMap = new Map<string, Agent>();
  workflow.agents?.forEach(agent => agentMap.set(agent.id, agent));

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-gradient-to-r from-black/20 to-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <FileText className="w-5 h-5 text-indigo-500" />
            工作流详情
          </h2>
          <div className="flex items-center gap-2">
            {isEditing ? (
              <>
                <button
                  onClick={handleSave}
                  className="btn-primary text-sm py-1.5"
                >
                  保存
                </button>
                <button
                  onClick={handleCancel}
                  className="btn-secondary text-sm py-1.5"
                >
                  取消
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={() => setIsEditing(true)}
                  className="btn-ghost text-sm gap-1.5"
                >
                  <Edit className="w-3.5 h-3.5" /> 编辑
                </button>
                <button
                  onClick={() => onEdit(workflow)}
                  className="btn-primary text-sm py-1.5"
                >
                  打开编辑器
                </button>
              </>
            )}
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Name & Version */}
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
                {workflow.name}
              </h3>
            )}
            <div className="flex items-center gap-2">
              <span className="px-2.5 py-1 rounded-full bg-indigo-500/10 text-indigo-600 text-xs font-medium border border-indigo-500/20">
                v{workflow.version}
              </span>
            </div>
          </div>

          {/* Description */}
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
                {workflow.description || '暂无描述'}
              </p>
            )}
          </div>

          {/* Stages */}
          <div className="space-y-3">
            <h4 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <GitBranch className="w-4 h-4" />
              阶段 ({workflow.stage_count ?? workflow.stages?.length ?? 0})
            </h4>
            <div className="space-y-3">
              {workflow.stages?.map((stage, index) => (
                <div key={index} className="bg-gradient-to-r from-indigo-500/5 to-purple-500/5 rounded-xl p-4 border border-indigo-500/10">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold">{stage.name}</span>
                    {stage.parallel && (
                      <span className="px-2 py-0.5 text-xs bg-blue-500/10 text-blue-600 rounded-full border border-blue-500/20">
                        并行
                      </span>
                    )}
                  </div>
                  <div className="mt-3 flex flex-wrap gap-2">
                    {stage.agents.map((agentRole) => (
                      <span
                        key={agentRole}
                        className="px-2.5 py-1 text-xs bg-card rounded-lg border border-border"
                      >
                        {agentRole}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Agents */}
          <div className="space-y-3">
            <h4 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Users className="w-4 h-4" />
              智能体 ({workflow.agent_count ?? workflow.agents?.length ?? 0})
            </h4>
            <div className="space-y-3">
              {workflow.agents?.map((agent) => {
                const dependsOnAgents = agent.depends_on
                  .map(id => agentMap.get(id)?.role)
                  .filter(Boolean);
                return (
                  <div key={agent.id} className="bg-gradient-to-r from-purple-500/5 to-pink-500/5 rounded-xl p-4 border border-purple-500/10">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="font-semibold capitalize">{agent.role}</span>
                      <span className="px-2 py-0.5 text-xs bg-gradient-to-r from-indigo-500/10 to-purple-500/10 rounded-full border border-indigo-500/20 text-indigo-600">
                        {agent.model}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground/80 line-clamp-2 mb-2">
                      {agent.prompt}
                    </p>
                    {dependsOnAgents.length > 0 && (
                      <div className="flex items-center gap-2 text-xs">
                        <span className="text-muted-foreground">依赖:</span>
                        {dependsOnAgents.map((role, i) => (
                          <span key={i} className="px-2 py-0.5 bg-card rounded border border-border">
                            {role}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Timestamps */}
          <div className="pt-4 border-t border-border/50">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Clock className="w-3.5 h-3.5" />
              <span>创建: {workflow.created_at ? new Date(workflow.created_at).toLocaleString('zh-CN') : '未知'}</span>
            </div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
              <Clock className="w-3.5 h-3.5" />
              <span>更新: {workflow.updated_at ? new Date(workflow.updated_at).toLocaleString('zh-CN') : '未知'}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// 示例工作流数据
const SAMPLE_WORKFLOWS: Workflow[] = [
  {
    id: 'sample-1',
    name: '简单规划',
    version: '1.0',
    description: '单智能体任务执行',
    stages: [{ name: '规划', agents: ['planner'], parallel: false }],
    agents: [{ id: 'a1', role: 'planner', model: 'claude-opus-4-6', prompt: 'You are a helpful planner.', depends_on: [] }],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'sample-2',
    name: '多智能体并行',
    version: '1.0',
    description: '多智能体并行执行',
    stages: [{ name: '并行任务', agents: ['dev', 'reviewer', 'tester'], parallel: true }],
    agents: [
      { id: 'a1', role: 'developer', model: 'claude-sonnet-4-6', prompt: 'You are a developer.', depends_on: [] },
      { id: 'a2', role: 'reviewer', model: 'claude-sonnet-4-6', prompt: 'You are a reviewer.', depends_on: [] },
      { id: 'a3', role: 'tester', model: 'claude-sonnet-4-6', prompt: 'You are a tester.', depends_on: [] },
    ],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'sample-3',
    name: 'TDD 工作流',
    version: '1.0',
    description: '测试驱动开发流程',
    stages: [
      { name: '编写测试', agents: ['tester'], parallel: false },
      { name: '实现', agents: ['developer'], parallel: false },
      { name: '审查', agents: ['reviewer'], parallel: false },
    ],
    agents: [
      { id: 'a1', role: 'tester', model: 'claude-haiku-4-5', prompt: 'Write a failing test.', depends_on: [] },
      { id: 'a2', role: 'developer', model: 'claude-opus-4-6', prompt: 'Implement the feature.', depends_on: ['a1'] },
      { id: 'a3', role: 'reviewer', model: 'claude-sonnet-4-6', prompt: 'Review the code.', depends_on: ['a2'] },
    ],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

export function WorkflowsPage() {
  const navigate = useNavigate();
  const { getWorkflow, deleteWorkflow, setCurrentWorkflow, createWorkflow } = useWorkflowStore();
  const { currentWorkspace } = useWorkspaceStore();
  const [displayWorkflows, setDisplayWorkflows] = useState<Workflow[]>([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState<Workflow | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [showSkillModal, setShowSkillModal] = useState(false);
  const [tutorialWorkflow, setTutorialWorkflow] = useState<Workflow | null>(null);
  const [tutorialLoading, setTutorialLoading] = useState<string | null>(null);
  const [activeCategory, setActiveCategory] = useState<WorkflowCategory>('all');
  const [launchWorkflow, setLaunchWorkflow] = useState<Workflow | null>(null);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  // Use React Query for fetching
  const { workflows, loading, refetch } = useWorkflowsQuery();

  // Listen for workspace changes
  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => { unsubscribe(); };
  }, [refetch]);

  useEffect(() => {
    if (!loading) {
      if (workflows.length === 0) {
        setDisplayWorkflows(SAMPLE_WORKFLOWS);
      } else {
        setDisplayWorkflows(workflows);
      }
    }
  }, [loading, workflows]);

  const handleCreate = () => {
    navigate('/editor');
  };

  const handleCreateFromSkill = async (skill: SkillSummary) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/skills/${skill.id}/generate-workflow`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      });

      if (response.ok) {
        const data = await response.json();
        // Refresh workflows list using React Query
        refetch();
        // Navigate to the new workflow or show success
        showSuccess(`工作流 "${data.name}" 创建成功！`);
      } else {
        const error = await response.json();
        showError(`创建失败: ${error.error || '未知错误'}`);
      }
    } catch (error) {
      console.error('Failed to create workflow from skill:', error);
      showError(`创建失败: ${error}`);
    }
  };

  const handleEdit = (workflow: Workflow) => {
    setCurrentWorkflow(workflow);
    navigate('/editor');
  };

  const handleCardClick = async (workflow: Workflow, e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button')) return;

    // 如果工作流没有完整数据（stages为空），则获取完整详情
    if (!workflow.stages || workflow.stages.length === 0) {
      setDetailLoading(true);
      const fullWorkflow = await getWorkflow(workflow.id);
      setSelectedWorkflow(fullWorkflow);
      setDetailLoading(false);
    } else {
      setSelectedWorkflow(workflow);
    }
  };

  const [executingIds] = useState<Set<string>>(new Set());

  const handleShowTutorial = async (workflow: Workflow, e: React.MouseEvent) => {
    e.stopPropagation();
    // 如果已有完整数据（stages 不为空），直接展示
    if (workflow.stages.length > 0) {
      setTutorialWorkflow(workflow);
      return;
    }
    // 否则先拉取完整详情
    setTutorialLoading(workflow.id);
    const full = await getWorkflow(workflow.id);
    setTutorialLoading(null);
    if (full) setTutorialWorkflow(full);
  };

  const handleExecute = (workflow: Workflow, e: React.MouseEvent) => {
    e.stopPropagation();
    setLaunchWorkflow(workflow);
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
              工作流
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">管理您的工作流和智能体编排</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowSkillModal(true)}
            className="btn-secondary"
          >
            <Wand2 className="w-4 h-4" />
            从技能创建
          </button>
          <button onClick={handleCreate} className="btn-primary">
            <Plus className="w-4 h-4" />
            新建工作流
          </button>
        </div>
      </div>

      <OperationGuide />

      {/* 分类 Tab */}
      <div className="flex items-center gap-2 flex-wrap">
        {(['all', 'dev', 'issue', 'ui-design'] as WorkflowCategory[]).map(cat => {
          const count = cat === 'all' ? displayWorkflows.length : displayWorkflows.filter(w => getWorkflowCategory(w.name) === cat).length;
          return (
            <button
              key={cat}
              onClick={() => setActiveCategory(cat)}
              className={cn(
                'flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-sm font-medium transition-all border',
                activeCategory === cat
                  ? 'bg-primary/10 text-primary border-primary/30 shadow-sm'
                  : 'border-border/50 hover:bg-accent text-muted-foreground'
              )}
            >
              {CATEGORY_ICONS[cat]}
              {CATEGORY_LABELS[cat]}
              <span className={cn('ml-0.5 text-xs px-1.5 py-0.5 rounded-full', activeCategory === cat ? 'bg-primary/20' : 'bg-muted')}>{count}</span>
            </button>
          );
        })}
        {activeCategory === 'issue' && (
          <button onClick={() => navigate('/tasks')} className="ml-auto flex items-center gap-1.5 text-xs text-indigo-600 hover:underline">
            <Bug className="w-3.5 h-3.5" />前往 Issue 管理页
          </button>
        )}
        {activeCategory === 'ui-design' && (
          <button onClick={() => navigate('/ui-design')} className="ml-auto flex items-center gap-1.5 text-xs text-blue-600 hover:underline">
            <Palette className="w-3.5 h-3.5" />前往 UI 设计工作台
          </button>
        )}
      </div>

      {displayWorkflows.length === 0 ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <GitBranch className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无工作流</h3>
          <p className="text-muted-foreground mb-4">创建您的第一个工作流开始</p>
          <button onClick={handleCreate} className="btn-primary">
            <Plus className="w-4 h-4" />
            新建工作流
          </button>
        </div>
      ) : (
        <div className="grid gap-4 stagger-children">
          {displayWorkflows.filter(w => activeCategory === 'all' || getWorkflowCategory(w.name) === activeCategory).map((workflow) => (
            <div
              key={workflow.id}
              className={cn(
                'bg-card rounded-2xl border border-border/50 p-5',
                'hover:shadow-lg hover:shadow-primary/5 hover:border-primary/20',
                'transition-all duration-200 cursor-pointer group',
                'hover:-translate-y-0.5'
              )}
              onClick={(e) => handleCardClick(workflow, e)}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-2">
                    <h3 className="font-semibold text-lg group-hover:text-indigo-600 transition-colors">
                      {workflow.name}
                    </h3>
                    <span className="px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-600 text-xs font-medium">
                      v{workflow.version}
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground mb-3">
                    {workflow.description || '无描述'}
                  </p>

                  <div className="flex items-center gap-3 flex-wrap">
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-gradient-to-r from-indigo-500/10 to-purple-500/10 text-xs font-medium text-indigo-600 border border-indigo-500/20">
                      <GitBranch className="w-3 h-3" />
                      {workflow.stage_count ?? workflow.stages?.length ?? 0} 阶段
                    </span>
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-gradient-to-r from-purple-500/10 to-pink-500/10 text-xs font-medium text-purple-600 border border-purple-500/20">
                      <Users className="w-3 h-3" />
                      {workflow.agent_count ?? workflow.agents?.length ?? 0} 智能体
                    </span>
                    <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                      <Clock className="w-3 h-3" />
                      {workflow.updated_at ? new Date(workflow.updated_at).toLocaleString('zh-CN', {
                        month: 'short',
                        day: 'numeric',
                        hour: '2-digit',
                        minute: '2-digit'
                      }) : '未知'}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-2 ml-4" onClick={(e) => e.stopPropagation()}>
                  <button
                    onClick={(e) => handleShowTutorial(workflow, e)}
                    disabled={tutorialLoading === workflow.id}
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      'bg-gradient-to-r from-sky-500 to-blue-500',
                      'text-white shadow-lg shadow-sky-500/25',
                      'hover:shadow-sky-500/40 hover:-translate-y-0.5 active:translate-y-0'
                    )}
                    title="查看使用教程"
                  >
                    {tutorialLoading === workflow.id ? (
                      <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                    ) : (
                      <Eye className="w-4 h-4" />
                    )}
                  </button>
                  <button
                    onClick={(e) => handleExecute(workflow, e)}
                    disabled={executingIds.has(workflow.id)}
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      executingIds.has(workflow.id)
                        ? 'bg-gray-400 cursor-not-allowed'
                        : 'bg-gradient-to-r from-emerald-500 to-green-500 hover:shadow-emerald-500/40 hover:-translate-y-0.5 active:translate-y-0',
                      'text-white shadow-lg shadow-emerald-500/25'
                    )}
                    title="执行"
                  >
                    {executingIds.has(workflow.id) ? (
                      <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                    ) : (
                      <Play className="w-4 h-4" />
                    )}
                  </button>
                  <button
                    onClick={() => handleEdit(workflow)}
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      'bg-gradient-to-r from-indigo-500 to-purple-500',
                      'text-white shadow-lg shadow-indigo-500/25',
                      'hover:shadow-indigo-500/40 hover:-translate-y-0.5 active:translate-y-0'
                    )}
                    title="编辑"
                  >
                    <Edit className="w-4 h-4" />
                  </button>
                  <button
                    className={cn(
                      'p-2.5 rounded-xl transition-all duration-200',
                      'hover:bg-red-500/10 text-muted-foreground hover:text-red-500',
                      'hover:-translate-y-0.5'
                    )}
                    title="删除"
                    onClick={() => {
                      showConfirm(
                        '删除工作流',
                        `确定删除工作流 "${workflow.name}"？`,
                        () => deleteWorkflow(workflow.id),
                        'danger'
                      );
                    }}
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {detailLoading && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/20">
          <div className="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full" />
        </div>
      )}

      {selectedWorkflow && !detailLoading && (
        <WorkflowDetailPanel
          workflow={selectedWorkflow}
          onClose={() => setSelectedWorkflow(null)}
          onEdit={handleEdit}
        />
      )}

      <SkillSelectorModal
        isOpen={showSkillModal}
        onClose={() => setShowSkillModal(false)}
        onSelect={handleCreateFromSkill}
      />

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

      {tutorialWorkflow && (
        <WorkflowTutorialModal
          workflow={tutorialWorkflow}
          onClose={() => setTutorialWorkflow(null)}
        />
      )}

      {launchWorkflow && (
        <WorkflowLaunchModal
          workflow={launchWorkflow}
          onClose={() => setLaunchWorkflow(null)}
        />
      )}
    </div>
  );
}
