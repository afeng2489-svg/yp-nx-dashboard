import { useEffect, useState } from 'react';
import {
  useSkillStore,
  type SkillSummary,
  type CreateSkillRequest,
  type UpdateSkillRequest,
} from '@/stores/skillStore';
import {
  useSkillsQuery,
  useSkillDetailQuery,
  useSkillCategoriesQuery,
} from '@/hooks/useReactQuery';
import { showSuccess, showError } from '@/lib/toast';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { Pencil, Trash2, Plus, X, Download, Link, FileText, ClipboardPaste } from 'lucide-react';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

export default function SkillsPage() {
  const {
    currentSkill,
    searchResults,
    stats,
    saving,
    executing,
    error,
    searchSkills,
    executeSkill,
    createSkill,
    updateSkill,
    deleteSkill,
    toggleSkillEnabled,
    importSkill,
    fetchStats,
    clearSearch,
    clearCurrentSkill,
  } = useSkillStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedSkill, setSelectedSkill] = useState<SkillSummary | null>(null);
  const [showExecuteDialog, setShowExecuteDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [executionResult, setExecutionResult] = useState<string | null>(null);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  // Import dialog state
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importMode, setImportMode] = useState<'url' | 'file' | 'paste'>('url');
  const [importContent, setImportContent] = useState('');
  const [importFilename, setImportFilename] = useState('');
  const [importing, setImporting] = useState(false);
  const [importPreview, setImportPreview] = useState<{
    name: string;
    description: string;
    category: string;
    tags: string[];
  } | null>(null);

  // Edit form state
  const [editForm, setEditForm] = useState<CreateSkillRequest>({
    id: '',
    name: '',
    description: '',
    category: 'development',
    tags: [],
    parameters: [],
  });

  // Use React Query for fetching
  const { skills, loading: skillsLoading, refetch: refetchSkills } = useSkillsQuery();
  const { categories, loading: categoriesLoading } = useSkillCategoriesQuery();
  useSkillDetailQuery(selectedSkill?.id || null);

  // Get skills directly from store when filtered
  const filteredSkills = useSkillStore((s) => s.skills);

  // Combined loading state
  const isLoading = skillsLoading || categoriesLoading;

  // Fetch stats on mount (not managed by React Query)
  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  // Update selected skill detail when it changes
  useEffect(() => {
    if (currentSkill && selectedSkill?.id === currentSkill.id) {
      // Already have this skill
    } else if (selectedSkill) {
      // Fetch skill detail if we only have summary
    }
  }, [selectedSkill, currentSkill]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      searchSkills(searchQuery);
      setSelectedCategory(null);
    }
  };

  const handleCategoryClick = (category: string) => {
    if (selectedCategory === category) {
      setSelectedCategory('');
      setSearchQuery('');
      clearSearch();
      refetchSkills();
    } else {
      setSelectedCategory(category);
      setSearchQuery('');
      clearSearch();
      useSkillStore.getState().fetchByCategory(category);
    }
  };

  const handleSkillClick = async (skill: SkillSummary) => {
    setSelectedSkill(skill);
    const detail = await useSkillStore.getState().fetchSkill(skill.id);
    if (detail) {
      setSelectedSkill(skill);
    }
  };

  const handleOpenCreateDialog = () => {
    setIsCreating(true);
    setEditForm({
      id: '',
      name: '',
      description: '',
      category: 'development',
      tags: [],
      parameters: [],
    });
    setShowEditDialog(true);
  };

  const handleOpenEditDialog = () => {
    if (!currentSkill) return;
    setIsCreating(false);
    setEditForm({
      id: currentSkill.id,
      name: currentSkill.name,
      description: currentSkill.description,
      category: currentSkill.category,
      tags: currentSkill.tags,
      parameters: currentSkill.parameters,
      code: currentSkill.code || '',
    });
    setShowEditDialog(true);
  };

  const handleCloseEditDialog = () => {
    setShowEditDialog(false);
    setEditForm({
      id: '',
      name: '',
      description: '',
      category: 'development',
      tags: [],
      parameters: [],
      code: '',
    });
  };

  const handleSaveSkill = async () => {
    if (!editForm.id.trim() || !editForm.name.trim()) {
      showError('验证失败', 'ID 和名称不能为空');
      return;
    }

    let result;
    if (isCreating) {
      const createData: CreateSkillRequest = {
        ...editForm,
        code: editForm.code,
      };
      result = await createSkill(createData);
      if (result) {
        showSuccess('创建成功', `技能 "${result.name}" 已创建`);
      } else {
        showError('创建失败', useSkillStore.getState().error || '未知错误');
        return;
      }
    } else {
      const updateData: UpdateSkillRequest = {
        name: editForm.name,
        description: editForm.description,
        category: editForm.category,
        tags: editForm.tags,
        code: editForm.code,
      };
      result = await updateSkill(editForm.id, updateData);
      if (result) {
        showSuccess('更新成功', `技能 "${result.name}" 已更新`);
      } else {
        showError('更新失败', useSkillStore.getState().error || '未知错误');
        return;
      }
    }

    handleCloseEditDialog();
    refetchSkills();
    fetchStats();
  };

  const handleDeleteSkill = () => {
    if (!currentSkill) return;
    showConfirm(
      '删除技能',
      `确定要删除技能「${currentSkill.name}」吗？此操作不可撤销。`,
      async () => {
        const success = await deleteSkill(currentSkill.id);
        if (success) {
          showSuccess('删除成功', `技能 "${currentSkill.name}" 已删除`);
          clearCurrentSkill();
          setSelectedSkill(null);
          refetchSkills();
          fetchStats();
        } else {
          showError('删除失败', '请稍后重试');
        }
      },
    );
  };

  // === 导入功能 ===
  const parseFrontmatter = (content: string) => {
    const lines = content.split('\n');
    let inFrontmatter = false;
    let name = '';
    let description = '';
    let category = 'general';
    let tags: string[] = ['agent'];

    for (const line of lines) {
      if (line.trim() === '---') {
        if (!inFrontmatter) {
          inFrontmatter = true;
          continue;
        } else break;
      }
      if (!inFrontmatter) continue;
      const match = line.match(/^(\w+):\s*(.+)/);
      if (!match) continue;
      const [, key, value] = match;
      if (key === 'name') name = value.trim();
      else if (key === 'description') description = value.trim();
      else if (key === 'category') category = value.trim();
      else if (key === 'tags') {
        try {
          tags = JSON.parse(value.trim());
        } catch {
          tags = value.split(',').map((t) => t.trim());
        }
      }
    }
    return name ? { name, description, category, tags } : null;
  };

  const handleImportContentChange = (content: string) => {
    setImportContent(content);
    if (importMode === 'paste' || importMode === 'file') {
      setImportPreview(parseFrontmatter(content));
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setImportFilename(file.name);
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setImportContent(text);
      setImportPreview(parseFrontmatter(text));
    };
    reader.readAsText(file);
  };

  const handleFileDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (!file || !file.name.endsWith('.md')) return;
    setImportFilename(file.name);
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setImportContent(text);
      setImportPreview(parseFrontmatter(text));
    };
    reader.readAsText(file);
  };

  const handleImport = async () => {
    if (!importContent.trim()) {
      showError('导入失败', '内容不能为空');
      return;
    }
    setImporting(true);
    const result = await importSkill(importMode, importContent, importFilename || undefined);
    setImporting(false);
    if (result) {
      showSuccess('导入成功', `技能 "${result.name}" 已导入`);
      setShowImportDialog(false);
      setImportContent('');
      setImportFilename('');
      setImportPreview(null);
      refetchSkills();
      fetchStats();
    } else {
      showError('导入失败', useSkillStore.getState().error || '未知错误');
    }
  };

  const handleOpenExecuteDialog = () => {
    if (!currentSkill) return;
    const initialValues: Record<string, string> = {};
    currentSkill.parameters.forEach((param) => {
      if (param.default !== null && param.default !== undefined) {
        initialValues[param.name] = String(param.default);
      } else {
        initialValues[param.name] = '';
      }
    });
    setParamValues(initialValues);
    setExecutionResult(null);
    setShowExecuteDialog(true);
  };

  const handleCloseExecuteDialog = () => {
    setShowExecuteDialog(false);
    setParamValues({});
    setExecutionResult(null);
  };

  const handleExecuteSkill = async () => {
    if (!currentSkill) return;

    const params: Record<string, unknown> = {};
    currentSkill.parameters.forEach((param) => {
      const value = paramValues[param.name];
      if (value !== undefined && value !== '') {
        try {
          params[param.name] = JSON.parse(value);
        } catch {
          params[param.name] = value;
        }
      }
    });

    try {
      const result = await executeSkill({
        skill_id: currentSkill.id,
        params,
      });

      if (result.success) {
        showSuccess('技能执行成功', `耗时: ${result.duration_ms}ms`);
        setExecutionResult(
          result.output !== null ? JSON.stringify(result.output, null, 2) : '执行完成，无输出',
        );
      } else {
        showError('技能执行失败', result.error || '未知错误');
        setExecutionResult(`执行失败: ${result.error}`);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : '执行出错';
      showError('技能执行失败', message);
      setExecutionResult(`执行失败: ${message}`);
    }
  };

  const displaySkills = searchQuery.trim()
    ? searchResults
    : selectedCategory
      ? filteredSkills
      : skills;

  // Common input class
  const inputCls =
    'w-full px-3 py-2 border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary bg-background text-foreground';

  return (
    <>
      <div className="flex h-full">
        {/* 左侧列表 */}
        <div className="w-80 border-r border-border flex flex-col bg-card">
          {/* 搜索框 */}
          <form onSubmit={handleSearch} className="p-4 border-b border-border">
            <div className="relative">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索技能..."
                className={inputCls + ' pl-10'}
              />
              <svg
                className="absolute left-3 top-2.5 w-5 h-5 text-muted-foreground"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
            </div>
          </form>

          {/* 新建 & 导入按钮 */}
          <div className="p-4 border-b border-border flex gap-2">
            <button
              onClick={handleOpenCreateDialog}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 btn-primary rounded-lg"
            >
              <Plus className="w-4 h-4" />
              新建
            </button>
            <button
              onClick={() => {
                setShowImportDialog(true);
                setImportMode('url');
                setImportContent('');
                setImportFilename('');
                setImportPreview(null);
              }}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition-colors"
            >
              <Download className="w-4 h-4" />
              导入
            </button>
          </div>

          {/* 统计信息 */}
          {stats && (
            <div className="p-4 border-b border-border bg-accent/50">
              <div className="text-sm text-muted-foreground">
                共 <span className="font-semibold text-primary">{stats.total_skills}</span> 个技能
              </div>
            </div>
          )}

          {/* 类别标签 */}
          {categories.length > 0 && (
            <div className="p-4 border-b border-border">
              <h3 className="text-sm font-medium text-foreground mb-2">类别</h3>
              <div className="flex flex-wrap gap-2">
                {categories.map((cat) => (
                  <button
                    key={cat}
                    onClick={() => handleCategoryClick(cat)}
                    className={`px-2 py-1 text-xs rounded-full transition-colors ${
                      selectedCategory === cat
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-accent text-muted-foreground hover:bg-accent/80'
                    }`}
                  >
                    {cat.replace('_', ' ')}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 技能列表 */}
          <div className="flex-1 overflow-y-auto">
            {isLoading ? (
              <div className="p-4 text-center text-muted-foreground">加载中...</div>
            ) : error ? (
              <div className="p-4 text-center text-destructive">{error}</div>
            ) : displaySkills.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground">没有找到技能</div>
            ) : (
              <div className="divide-y divide-border">
                {displaySkills.map((skill) => (
                  <button
                    key={skill.id}
                    onClick={() => handleSkillClick(skill)}
                    className={`w-full p-4 text-left hover:bg-accent transition-colors ${
                      selectedSkill?.id === skill.id ? 'bg-primary/5' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="font-medium text-foreground">{skill.name}</div>
                      {skill.is_preset && (
                        <span className="px-2 py-0.5 text-xs bg-purple-500/10 text-purple-500 rounded">
                          预设
                        </span>
                      )}
                    </div>
                    <div className="text-sm text-muted-foreground mt-1 line-clamp-2">
                      {skill.description}
                    </div>
                    <div className="flex items-center gap-2 mt-2">
                      <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
                        {skill.category.replace('_', ' ')}
                      </span>
                      {skill.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className="px-2 py-0.5 text-xs bg-primary/10 text-primary rounded"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* 右侧详情 */}
        <div className="flex-1 bg-card overflow-y-auto">
          {currentSkill ? (
            <div className="p-6">
              {/* 头部操作按钮 */}
              <div className="flex items-center justify-between mb-6">
                <div>
                  <h1 className="text-2xl font-bold text-foreground">{currentSkill.name}</h1>
                  <div className="flex items-center gap-2 mt-1">
                    <p className="text-muted-foreground">版本 {currentSkill.version}</p>
                    {currentSkill.is_preset && (
                      <span className="px-2 py-0.5 text-xs bg-purple-500/10 text-purple-500 rounded">
                        预设技能
                      </span>
                    )}
                    {!currentSkill.enabled && (
                      <span className="px-2 py-0.5 text-xs bg-destructive/10 text-destructive rounded">
                        已禁用
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => toggleSkillEnabled(currentSkill.id, !currentSkill.enabled)}
                    className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      currentSkill.enabled ? 'bg-primary' : 'bg-muted'
                    }`}
                    title={currentSkill.enabled ? '点击禁用' : '点击启用'}
                  >
                    <span
                      className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                        currentSkill.enabled ? 'translate-x-6' : 'translate-x-1'
                      }`}
                    />
                  </button>
                  <button
                    onClick={handleOpenEditDialog}
                    className="flex items-center gap-1 px-3 py-1.5 text-primary hover:bg-primary/10 rounded-lg transition-colors"
                  >
                    <Pencil className="w-4 h-4" />
                    编辑
                  </button>
                  {!currentSkill.is_preset && (
                    <button
                      onClick={handleDeleteSkill}
                      className="flex items-center gap-1 px-3 py-1.5 text-destructive hover:bg-destructive/10 rounded-lg transition-colors"
                    >
                      <Trash2 className="w-4 h-4" />
                      删除
                    </button>
                  )}
                </div>
              </div>

              <div className="mb-6">
                <h2 className="text-sm font-medium text-muted-foreground mb-2">描述</h2>
                <p className="text-foreground">{currentSkill.description}</p>
              </div>

              <div className="mb-6">
                <h2 className="text-sm font-medium text-muted-foreground mb-2">标签</h2>
                <div className="flex flex-wrap gap-2">
                  {currentSkill.tags.map((tag) => (
                    <span
                      key={tag}
                      className="px-3 py-1 text-sm bg-primary/10 text-primary rounded-full"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>

              {currentSkill.parameters.length > 0 && (
                <div className="mb-6">
                  <h2 className="text-sm font-medium text-muted-foreground mb-3">参数</h2>
                  <div className="space-y-3">
                    {currentSkill.parameters.map((param) => (
                      <div
                        key={param.name}
                        className="p-3 bg-accent/50 rounded-lg border border-border"
                      >
                        <div className="flex items-center gap-2">
                          <span className="font-medium text-foreground">{param.name}</span>
                          {param.required ? (
                            <span className="px-2 py-0.5 text-xs bg-destructive/10 text-destructive rounded">
                              必需
                            </span>
                          ) : (
                            <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
                              可选
                            </span>
                          )}
                          <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
                            {param.param_type}
                          </span>
                        </div>
                        <p className="text-sm text-muted-foreground mt-1">{param.description}</p>
                        {param.default !== null && (
                          <p className="text-xs text-muted-foreground mt-1">
                            默认值: {JSON.stringify(param.default)}
                          </p>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* 技能内容 */}
              {currentSkill.code && (
                <div className="mb-6">
                  <h2 className="text-sm font-medium text-muted-foreground mb-2">技能内容</h2>
                  <pre className="w-full p-4 bg-background border border-border rounded-lg text-sm text-foreground whitespace-pre-wrap overflow-x-auto max-h-96 overflow-y-auto">
                    {currentSkill.code}
                  </pre>
                </div>
              )}

              {/* 执行按钮 */}
              <div className="mt-8 pt-6 border-t border-border">
                <button
                  onClick={handleOpenExecuteDialog}
                  disabled={executing}
                  className="px-6 py-2 btn-primary rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {executing ? '执行中...' : '执行技能'}
                </button>
              </div>
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <svg
                  className="w-16 h-16 mx-auto mb-4 text-muted-foreground/30"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={1.5}
                    d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
                  />
                </svg>
                <p>选择一个技能查看详情</p>
              </div>
            </div>
          )}
        </div>

        {/* 编辑弹窗 */}
        {showEditDialog && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
              <div className="p-6 border-b border-border flex items-center justify-between">
                <div>
                  <h2 className="text-xl font-bold text-foreground">
                    {isCreating ? '新建技能' : '编辑技能'}
                  </h2>
                  {!isCreating && (
                    <p className="text-sm text-muted-foreground mt-1">ID: {editForm.id}</p>
                  )}
                </div>
                <button onClick={handleCloseEditDialog} className="p-1 hover:bg-accent rounded">
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="p-6 overflow-y-auto flex-1 space-y-4">
                {isCreating && (
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      技能 ID <span className="text-destructive">*</span>
                    </label>
                    <input
                      type="text"
                      value={editForm.id}
                      onChange={(e) => setEditForm({ ...editForm, id: e.target.value })}
                      placeholder="例如: my-custom-skill"
                      className={inputCls}
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      唯一标识符，只能使用字母、数字和连字符
                    </p>
                  </div>
                )}

                <div>
                  <label className="block text-sm font-medium text-foreground mb-1">
                    名称 <span className="text-destructive">*</span>
                  </label>
                  <input
                    type="text"
                    value={editForm.name}
                    onChange={(e) => setEditForm({ ...editForm, name: e.target.value })}
                    placeholder="技能显示名称"
                    className={inputCls}
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-foreground mb-1">描述</label>
                  <textarea
                    value={editForm.description}
                    onChange={(e) => setEditForm({ ...editForm, description: e.target.value })}
                    placeholder="技能的详细描述"
                    rows={3}
                    className={inputCls + ' resize-none'}
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-foreground mb-1">类别</label>
                  <Select
                    value={editForm.category}
                    onValueChange={(v) => setEditForm({ ...editForm, category: v })}
                  >
                    <SelectTrigger className="h-8 text-sm">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="workflow_planning">
                        工作流规划 (workflow_planning)
                      </SelectItem>
                      <SelectItem value="collaboration">协作 (collaboration)</SelectItem>
                      <SelectItem value="development">开发 (development)</SelectItem>
                      <SelectItem value="testing">测试 (testing)</SelectItem>
                      <SelectItem value="review">审查 (review)</SelectItem>
                      <SelectItem value="documentation">文档 (documentation)</SelectItem>
                      <SelectItem value="research">研究 (research)</SelectItem>
                      <SelectItem value="general">通用 (general)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <label className="block text-sm font-medium text-foreground mb-1">
                    标签 (逗号分隔)
                  </label>
                  <input
                    type="text"
                    value={editForm.tags?.join(', ') || ''}
                    onChange={(e) =>
                      setEditForm({
                        ...editForm,
                        tags: e.target.value
                          .split(',')
                          .map((t) => t.trim())
                          .filter(Boolean),
                      })
                    }
                    placeholder="例如: test, demo, api"
                    className={inputCls}
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-foreground mb-1">
                    技能内容 (instruction)
                  </label>
                  <textarea
                    value={editForm.code || ''}
                    onChange={(e) => setEditForm({ ...editForm, code: e.target.value })}
                    placeholder="技能的指令内容，支持多行 Markdown 格式"
                    rows={10}
                    className={inputCls + ' font-mono text-sm resize-none'}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    技能的完整指令内容，会作为 agent 的 system prompt
                  </p>
                </div>
              </div>

              <div className="p-6 border-t border-border flex justify-end gap-3">
                <button onClick={handleCloseEditDialog} className="btn-secondary">
                  取消
                </button>
                <button
                  onClick={handleSaveSkill}
                  disabled={saving}
                  className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {saving ? '保存中...' : '保存'}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* 执行弹窗 */}
        {showExecuteDialog && currentSkill && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
              <div className="p-6 border-b border-border">
                <h2 className="text-xl font-bold text-foreground">执行技能</h2>
                <p className="text-sm text-muted-foreground mt-1">{currentSkill.name}</p>
              </div>

              <div className="p-6 overflow-y-auto flex-1">
                {currentSkill.parameters.length > 0 ? (
                  <div className="space-y-4">
                    {currentSkill.parameters.map((param) => (
                      <div key={param.name}>
                        <label className="block text-sm font-medium text-foreground mb-1">
                          {param.name}
                          {param.required && <span className="text-destructive ml-1">*</span>}
                        </label>
                        <input
                          type="text"
                          value={paramValues[param.name] || ''}
                          onChange={(e) =>
                            setParamValues((prev) => ({
                              ...prev,
                              [param.name]: e.target.value,
                            }))
                          }
                          placeholder={
                            param.default !== null ? String(param.default) : param.description
                          }
                          className={inputCls}
                        />
                        <p className="text-xs text-muted-foreground mt-1">{param.description}</p>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-muted-foreground text-center py-4">此技能无需参数</p>
                )}

                {executionResult && (
                  <div className="mt-4">
                    <label className="block text-sm font-medium text-foreground mb-1">
                      执行结果
                    </label>
                    <pre className="w-full px-3 py-2 border border-border rounded-lg bg-background text-sm overflow-x-auto max-h-48">
                      {executionResult}
                    </pre>
                  </div>
                )}
              </div>

              <div className="p-6 border-t border-border flex justify-end gap-3">
                <button onClick={handleCloseExecuteDialog} className="btn-secondary">
                  关闭
                </button>
                <button
                  onClick={handleExecuteSkill}
                  disabled={executing}
                  className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {executing ? '执行中...' : '执行'}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* 导入弹窗 */}
        {showImportDialog && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
              <div className="p-6 border-b border-border flex items-center justify-between">
                <h2 className="text-xl font-bold text-foreground">导入技能</h2>
                <button
                  onClick={() => setShowImportDialog(false)}
                  className="p-1 hover:bg-accent rounded"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="p-6 overflow-y-auto flex-1 space-y-4">
                {/* 模式切换 */}
                <div className="flex gap-1 bg-accent rounded-lg p-1">
                  <button
                    onClick={() => {
                      setImportMode('url');
                      setImportContent('');
                      setImportPreview(null);
                    }}
                    className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      importMode === 'url'
                        ? 'bg-card text-primary shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    <Link className="w-4 h-4" />
                    URL
                  </button>
                  <button
                    onClick={() => {
                      setImportMode('file');
                      setImportContent('');
                      setImportPreview(null);
                      setImportFilename('');
                    }}
                    className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      importMode === 'file'
                        ? 'bg-card text-primary shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    <FileText className="w-4 h-4" />
                    文件
                  </button>
                  <button
                    onClick={() => {
                      setImportMode('paste');
                      setImportContent('');
                      setImportPreview(null);
                    }}
                    className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      importMode === 'paste'
                        ? 'bg-card text-primary shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    <ClipboardPaste className="w-4 h-4" />
                    粘贴
                  </button>
                </div>

                {/* URL 模式 */}
                {importMode === 'url' && (
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      .md 文件 URL
                    </label>
                    <input
                      type="url"
                      value={importContent}
                      onChange={(e) => setImportContent(e.target.value)}
                      placeholder="https://raw.githubusercontent.com/.../skill.md"
                      className={inputCls + ' text-sm'}
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      支持 GitHub raw 链接等可直接访问的 .md 文件 URL
                    </p>
                  </div>
                )}

                {/* 文件模式 */}
                {importMode === 'file' && (
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      选择 .md 文件
                    </label>
                    <div
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={handleFileDrop}
                      className="border-2 border-dashed border-border rounded-lg p-8 text-center hover:border-primary/50 transition-colors cursor-pointer"
                      onClick={() => document.getElementById('import-file-input')?.click()}
                    >
                      <input
                        id="import-file-input"
                        type="file"
                        accept=".md"
                        onChange={handleFileSelect}
                        className="hidden"
                      />
                      {importFilename ? (
                        <div>
                          <FileText className="w-8 h-8 mx-auto text-primary mb-2" />
                          <p className="text-sm font-medium text-foreground">{importFilename}</p>
                          <p className="text-xs text-muted-foreground mt-1">点击更换文件</p>
                        </div>
                      ) : (
                        <div>
                          <Download className="w-8 h-8 mx-auto text-muted-foreground mb-2" />
                          <p className="text-sm text-muted-foreground">拖拽 .md 文件到此处</p>
                          <p className="text-xs text-muted-foreground/60 mt-1">或点击选择文件</p>
                        </div>
                      )}
                    </div>
                  </div>
                )}

                {/* 粘贴模式 */}
                {importMode === 'paste' && (
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      Markdown 内容
                    </label>
                    <textarea
                      value={importContent}
                      onChange={(e) => handleImportContentChange(e.target.value)}
                      placeholder={
                        '---\nname: My Skill\ndescription: 技能描述\ncategory: development\ntags: ["agent"]\ninstruction: |\n  技能指令内容...\n---'
                      }
                      rows={12}
                      className={inputCls + ' font-mono text-sm resize-none'}
                    />
                  </div>
                )}

                {/* 预览区 */}
                {importPreview && (
                  <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-lg p-4">
                    <h3 className="text-sm font-medium text-emerald-600 dark:text-emerald-400 mb-2">
                      解析预览
                    </h3>
                    <div className="space-y-1 text-sm">
                      <div>
                        <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                          名称:
                        </span>{' '}
                        {importPreview.name}
                      </div>
                      {importPreview.description && (
                        <div>
                          <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                            描述:
                          </span>{' '}
                          {importPreview.description}
                        </div>
                      )}
                      <div>
                        <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                          分类:
                        </span>{' '}
                        {importPreview.category}
                      </div>
                      <div className="flex items-center gap-1">
                        <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                          标签:
                        </span>
                        {importPreview.tags.map((tag) => (
                          <span
                            key={tag}
                            className="px-1.5 py-0.5 text-xs bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                )}
              </div>

              <div className="p-6 border-t border-border flex justify-end gap-3">
                <button onClick={() => setShowImportDialog(false)} className="btn-secondary">
                  取消
                </button>
                <button
                  onClick={handleImport}
                  disabled={importing || !importContent.trim()}
                  className="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  {importing ? '导入中...' : '导入'}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={confirmState.onConfirm}
        onCancel={hideConfirm}
      />
    </>
  );
}
