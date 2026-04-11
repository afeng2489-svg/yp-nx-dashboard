import { useEffect, useState } from 'react';
import { useSkillStore, type SkillSummary, type CreateSkillRequest, type UpdateSkillRequest } from '@/stores/skillStore';
import { useSkillsQuery, useSkillDetailQuery, useSkillCategoriesQuery } from '@/hooks/useReactQuery';
import { showSuccess, showError } from '@/lib/toast';
import { Pencil, Trash2, Plus, X } from 'lucide-react';

export default function SkillsPage() {
  const {
    currentSkill,
    searchResults,
    stats,
    loading,
    saving,
    executing,
    error,
    searchSkills,
    executeSkill,
    createSkill,
    updateSkill,
    deleteSkill,
    fetchStats,
    clearSearch,
    clearError,
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
  const { categories, loading: categoriesLoading, refetch: refetchCategories } = useSkillCategoriesQuery();
  const { refetch: refetchSkillDetail } = useSkillDetailQuery(selectedSkill?.id || null);

  // Get skills directly from store when filtered
  const filteredSkills = useSkillStore((s) => s.skills);

  // Combined loading state
  const isLoading = skillsLoading || categoriesLoading;

  // Initial data fetch
  useEffect(() => {
    refetchSkills();
    refetchCategories();
    fetchStats();
  }, [refetchSkills, refetchCategories, fetchStats]);

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
      // Toggle off - show all
      setSelectedCategory('');
      setSearchQuery('');
      clearSearch();
      refetchSkills();
    } else {
      setSelectedCategory(category);
      setSearchQuery('');
      clearSearch();
      // Fetch skills filtered by category
      useSkillStore.getState().fetchByCategory(category);
    }
  };

  const handleSkillClick = async (skill: SkillSummary) => {
    setSelectedSkill(skill);
    // Fetch full detail
    const detail = await useSkillStore.getState().fetchSkill(skill.id);
    if (detail) {
      setSelectedSkill(skill); // Keep the summary for display
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

  const handleDeleteSkill = async () => {
    if (!currentSkill) return;
    if (!confirm(`确定要删除技能 "${currentSkill.name}" 吗？`)) return;

    const success = await deleteSkill(currentSkill.id);
    if (success) {
      showSuccess('删除成功', `技能 "${currentSkill.name}" 已删除`);
      clearCurrentSkill();
      setSelectedSkill(null);
      refetchSkills();
      fetchStats();
    } else {
      showError('删除失败', useSkillStore.getState().error || '未知错误');
    }
  };

  const handleOpenExecuteDialog = () => {
    if (!currentSkill) return;
    // Initialize param values with defaults
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

    // Build params object
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
          result.output !== null
            ? JSON.stringify(result.output, null, 2)
            : '执行完成，无输出'
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

  return (
    <div className="flex h-full">
      {/* 左侧列表 */}
      <div className="w-80 border-r border-gray-200 flex flex-col bg-white">
        {/* 搜索框 */}
        <form onSubmit={handleSearch} className="p-4 border-b border-gray-200">
          <div className="relative">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="搜索技能..."
              className="w-full px-4 py-2 pl-10 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <svg
              className="absolute left-3 top-2.5 w-5 h-5 text-gray-400"
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

        {/* 新建按钮 */}
        <div className="p-4 border-b border-gray-200">
          <button
            onClick={handleOpenCreateDialog}
            className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors"
          >
            <Plus className="w-4 h-4" />
            新建技能
          </button>
        </div>

        {/* 统计信息 */}
        {stats && (
          <div className="p-4 border-b border-gray-200 bg-gray-50">
            <div className="text-sm text-gray-600">
              共 <span className="font-semibold text-blue-600">{stats.total_skills}</span> 个技能
            </div>
          </div>
        )}

        {/* 类别标签 */}
        {categories.length > 0 && (
          <div className="p-4 border-b border-gray-200">
            <h3 className="text-sm font-medium text-gray-700 mb-2">类别</h3>
            <div className="flex flex-wrap gap-2">
              {categories.map((cat) => (
                <button
                  key={cat}
                  onClick={() => handleCategoryClick(cat)}
                  className={`px-2 py-1 text-xs rounded-full transition-colors ${
                    selectedCategory === cat
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
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
            <div className="p-4 text-center text-gray-500">加载中...</div>
          ) : error ? (
            <div className="p-4 text-center text-red-500">{error}</div>
          ) : displaySkills.length === 0 ? (
            <div className="p-4 text-center text-gray-500">没有找到技能</div>
          ) : (
            <div className="divide-y divide-gray-100">
              {displaySkills.map((skill) => (
                <button
                  key={skill.id}
                  onClick={() => handleSkillClick(skill)}
                  className={`w-full p-4 text-left hover:bg-gray-50 transition-colors ${
                    selectedSkill?.id === skill.id ? 'bg-blue-50' : ''
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="font-medium text-gray-900">{skill.name}</div>
                    {skill.is_preset && (
                      <span className="px-2 py-0.5 text-xs bg-purple-100 text-purple-600 rounded">
                        预设
                      </span>
                    )}
                  </div>
                  <div className="text-sm text-gray-500 mt-1 line-clamp-2">
                    {skill.description}
                  </div>
                  <div className="flex items-center gap-2 mt-2">
                    <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">
                      {skill.category.replace('_', ' ')}
                    </span>
                    {skill.tags.slice(0, 2).map((tag) => (
                      <span
                        key={tag}
                        className="px-2 py-0.5 text-xs bg-blue-50 text-blue-600 rounded"
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
      <div className="flex-1 bg-white overflow-y-auto">
        {currentSkill ? (
          <div className="p-6">
            {/* 头部操作按钮 */}
            <div className="flex items-center justify-between mb-6">
              <div>
                <h1 className="text-2xl font-bold text-gray-900">{currentSkill.name}</h1>
                <div className="flex items-center gap-2 mt-1">
                  <p className="text-gray-500">版本 {currentSkill.version}</p>
                  {currentSkill.is_preset && (
                    <span className="px-2 py-0.5 text-xs bg-purple-100 text-purple-600 rounded">
                      预设技能
                    </span>
                  )}
                  {!currentSkill.enabled && (
                    <span className="px-2 py-0.5 text-xs bg-red-100 text-red-600 rounded">
                      已禁用
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleOpenEditDialog}
                  className="flex items-center gap-1 px-3 py-1.5 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                >
                  <Pencil className="w-4 h-4" />
                  编辑
                </button>
                {!currentSkill.is_preset && (
                  <button
                    onClick={handleDeleteSkill}
                    className="flex items-center gap-1 px-3 py-1.5 text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                    删除
                  </button>
                )}
              </div>
            </div>

            <div className="mb-6">
              <h2 className="text-sm font-medium text-gray-700 mb-2">描述</h2>
              <p className="text-gray-900">{currentSkill.description}</p>
            </div>

            <div className="mb-6">
              <h2 className="text-sm font-medium text-gray-700 mb-2">标签</h2>
              <div className="flex flex-wrap gap-2">
                {currentSkill.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-3 py-1 text-sm bg-blue-50 text-blue-600 rounded-full"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>

            {currentSkill.parameters.length > 0 && (
              <div className="mb-6">
                <h2 className="text-sm font-medium text-gray-700 mb-3">参数</h2>
                <div className="space-y-3">
                  {currentSkill.parameters.map((param) => (
                    <div
                      key={param.name}
                      className="p-3 bg-gray-50 rounded-lg border border-gray-200"
                    >
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-gray-900">{param.name}</span>
                        {param.required ? (
                          <span className="px-2 py-0.5 text-xs bg-red-100 text-red-600 rounded">
                            必需
                          </span>
                        ) : (
                          <span className="px-2 py-0.5 text-xs bg-gray-200 text-gray-600 rounded">
                            可选
                          </span>
                        )}
                        <span className="px-2 py-0.5 text-xs bg-gray-200 text-gray-600 rounded">
                          {param.param_type}
                        </span>
                      </div>
                      <p className="text-sm text-gray-600 mt-1">{param.description}</p>
                      {param.default !== null && (
                        <p className="text-xs text-gray-500 mt-1">
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
                <h2 className="text-sm font-medium text-gray-700 mb-2">技能内容</h2>
                <pre className="w-full p-4 bg-gray-50 border border-gray-200 rounded-lg text-sm text-gray-800 whitespace-pre-wrap overflow-x-auto max-h-96 overflow-y-auto">
                  {currentSkill.code}
                </pre>
              </div>
            )}

            {/* 执行按钮 */}
            <div className="mt-8 pt-6 border-t border-gray-200">
              <button
                onClick={handleOpenExecuteDialog}
                disabled={executing}
                className="px-6 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {executing ? '执行中...' : '执行技能'}
              </button>
            </div>
          </div>
        ) : (
          <div className="h-full flex items-center justify-center text-gray-400">
            <div className="text-center">
              <svg
                className="w-16 h-16 mx-auto mb-4 text-gray-300"
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
          <div className="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
            <div className="p-6 border-b border-gray-200 flex items-center justify-between">
              <div>
                <h2 className="text-xl font-bold text-gray-900">
                  {isCreating ? '新建技能' : '编辑技能'}
                </h2>
                {!isCreating && (
                  <p className="text-sm text-gray-500 mt-1">ID: {editForm.id}</p>
                )}
              </div>
              <button
                onClick={handleCloseEditDialog}
                className="p-1 hover:bg-gray-100 rounded"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="p-6 overflow-y-auto flex-1 space-y-4">
              {isCreating && (
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    技能 ID <span className="text-red-500">*</span>
                  </label>
                  <input
                    type="text"
                    value={editForm.id}
                    onChange={(e) => setEditForm({ ...editForm, id: e.target.value })}
                    placeholder="例如: my-custom-skill"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    唯一标识符，只能使用字母、数字和连字符
                  </p>
                </div>
              )}

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  名称 <span className="text-red-500">*</span>
                </label>
                <input
                  type="text"
                  value={editForm.name}
                  onChange={(e) => setEditForm({ ...editForm, name: e.target.value })}
                  placeholder="技能显示名称"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  描述
                </label>
                <textarea
                  value={editForm.description}
                  onChange={(e) => setEditForm({ ...editForm, description: e.target.value })}
                  placeholder="技能的详细描述"
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  类别
                </label>
                <select
                  value={editForm.category}
                  onChange={(e) => setEditForm({ ...editForm, category: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="workflow_planning">工作流规划 (workflow_planning)</option>
                  <option value="collaboration">协作 (collaboration)</option>
                  <option value="development">开发 (development)</option>
                  <option value="testing">测试 (testing)</option>
                  <option value="review">审查 (review)</option>
                  <option value="documentation">文档 (documentation)</option>
                  <option value="research">研究 (research)</option>
                  <option value="general">通用 (general)</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  标签 (逗号分隔)
                </label>
                <input
                  type="text"
                  value={editForm.tags?.join(', ') || ''}
                  onChange={(e) =>
                    setEditForm({
                      ...editForm,
                      tags: e.target.value.split(',').map((t) => t.trim()).filter(Boolean),
                    })
                  }
                  placeholder="例如: test, demo, api"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  技能内容 (instruction)
                </label>
                <textarea
                  value={editForm.code || ''}
                  onChange={(e) => setEditForm({ ...editForm, code: e.target.value })}
                  placeholder="技能的指令内容，支持多行 Markdown 格式"
                  rows={10}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
                />
                <p className="text-xs text-gray-500 mt-1">
                  技能的完整指令内容，会作为 agent 的 system prompt
                </p>
              </div>
            </div>

            <div className="p-6 border-t border-gray-200 flex justify-end gap-3">
              <button
                onClick={handleCloseEditDialog}
                className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleSaveSkill}
                disabled={saving}
                className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
          <div className="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
            <div className="p-6 border-b border-gray-200">
              <h2 className="text-xl font-bold text-gray-900">执行技能</h2>
              <p className="text-sm text-gray-500 mt-1">{currentSkill.name}</p>
            </div>

            <div className="p-6 overflow-y-auto flex-1">
              {currentSkill.parameters.length > 0 ? (
                <div className="space-y-4">
                  {currentSkill.parameters.map((param) => (
                    <div key={param.name}>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        {param.name}
                        {param.required && <span className="text-red-500 ml-1">*</span>}
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
                        placeholder={param.default !== null ? String(param.default) : param.description}
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      />
                      <p className="text-xs text-gray-500 mt-1">{param.description}</p>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-gray-500 text-center py-4">此技能无需参数</p>
              )}

              {executionResult && (
                <div className="mt-4">
                  <label className="block text-sm font-medium text-gray-700 mb-1">执行结果</label>
                  <pre className="w-full px-3 py-2 border border-gray-200 rounded-lg bg-gray-50 text-sm overflow-x-auto max-h-48">
                    {executionResult}
                  </pre>
                </div>
              )}
            </div>

            <div className="p-6 border-t border-gray-200 flex justify-end gap-3">
              <button
                onClick={handleCloseExecuteDialog}
                className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
              >
                关闭
              </button>
              <button
                onClick={handleExecuteSkill}
                disabled={executing}
                className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {executing ? '执行中...' : '执行'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
