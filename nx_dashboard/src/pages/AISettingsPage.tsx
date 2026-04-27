import { useEffect, useState } from 'react';
import {
  useAIConfigStore,
  ClaudeSwitchBackendInfo,
  ClaudeSwitchBackendConfig,
} from '@/stores/aiConfigStore';
import { AddModelMappingRequest, ClaudeCliConfigResponse, api } from '@/api/client';
import { ProviderGrid } from '@/components/provider/ProviderGrid';
import { showSuccess, showError } from '@/lib/toast';
import {
  Loader2,
  Plus,
  Check,
  X,
  Bot,
  ArrowRightLeft,
  Terminal,
  AlertTriangle,
  RefreshCw,
  Save,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// ── Claude CLI 路径配置 ────────────────────────────────────────────────────

function ClaudeCliPathSection() {
  const [config, setConfig] = useState<ClaudeCliConfigResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [detecting, setDetecting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [inputPath, setInputPath] = useState('');

  const refresh = async () => {
    try {
      const data = await api.getClaudeCliConfig();
      setConfig(data);
      setInputPath(data.path ?? '');
    } catch (e) {
      showError(`获取 Claude CLI 配置失败: ${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const handleDetect = async () => {
    setDetecting(true);
    try {
      const data = await api.detectClaudeCli();
      setConfig(data);
      setInputPath(data.path ?? '');
      if (data.path) {
        showSuccess(`检测到 Claude CLI: ${data.path}`);
      } else {
        showError('未检测到 Claude CLI，请手动指定路径或先安装');
      }
    } catch (e) {
      showError(`自动检测失败: ${(e as Error).message}`);
    } finally {
      setDetecting(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const trimmed = inputPath.trim();
      const data = await api.setClaudeCliPath(trimmed === '' ? null : trimmed);
      setConfig(data);
      setInputPath(data.path ?? '');
      showSuccess(trimmed ? '已保存自定义路径' : '已清除自定义路径，回退到自动检测');
    } catch (e) {
      showError(`保存失败: ${(e as Error).message}`);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="rounded-xl border bg-card p-6 flex items-center gap-2 text-muted-foreground">
        <Loader2 className="w-4 h-4 animate-spin" />
        <span className="text-sm">加载 Claude CLI 配置...</span>
      </div>
    );
  }

  const sourceLabel =
    config?.source === 'user' ? '用户配置' : config?.source === 'auto' ? '自动检测' : '未找到';
  const sourceColor =
    config?.source === 'none'
      ? 'text-red-500 bg-red-500/10 border-red-500/30'
      : config?.source === 'user'
        ? 'text-blue-500 bg-blue-500/10 border-blue-500/30'
        : 'text-emerald-500 bg-emerald-500/10 border-emerald-500/30';

  return (
    <div className="rounded-xl border bg-card p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
          <Terminal className="w-5 h-5 text-indigo-500" />
        </div>
        <div className="flex-1">
          <h2 className="text-lg font-semibold">Claude Code CLI 路径</h2>
          <p className="text-xs text-muted-foreground mt-0.5">
            工作流和团队对话依赖本地 Claude CLI；优先使用此处配置，否则自动从系统 PATH 搜索
          </p>
        </div>
        <span className={cn('px-2.5 py-1 rounded-full text-xs font-medium border', sourceColor)}>
          {sourceLabel}
        </span>
      </div>

      {config?.source === 'none' && (
        <div className="mb-4 p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-start gap-2">
          <AlertTriangle className="w-4 h-4 text-amber-500 flex-shrink-0 mt-0.5" />
          <div className="text-xs text-amber-700 dark:text-amber-300">
            {config.install_hint || '未检测到 Claude Code CLI'}
          </div>
        </div>
      )}

      <div className="space-y-3">
        <div>
          <label className="text-xs font-medium text-muted-foreground mb-1 block">
            可执行文件路径（留空 = 使用自动检测）
          </label>
          <input
            type="text"
            value={inputPath}
            onChange={(e) => setInputPath(e.target.value)}
            placeholder="/opt/homebrew/bin/claude 或 ~/.npm-global/bin/claude"
            className="w-full px-3 py-2 rounded-lg border bg-background text-sm font-mono focus:ring-2 focus:ring-indigo-500/30 focus:border-indigo-500"
          />
          {config?.source === 'auto' && config.path && (
            <p className="text-xs text-muted-foreground mt-1">
              当前自动检测到：<span className="font-mono">{config.path}</span>
            </p>
          )}
        </div>

        <div className="flex gap-2">
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-4 py-2 rounded-lg bg-indigo-500 text-white text-sm font-medium hover:bg-indigo-600 disabled:opacity-50 flex items-center gap-2"
          >
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
            保存
          </button>
          <button
            onClick={handleDetect}
            disabled={detecting}
            className="px-4 py-2 rounded-lg border bg-background text-sm font-medium hover:bg-accent disabled:opacity-50 flex items-center gap-2"
          >
            {detecting ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            自动检测
          </button>
        </div>
      </div>
    </div>
  );
}

const CLAUDE_SWITCH_BACKENDS = [
  {
    id: 'minimax',
    name: 'MiniMax',
    defaultModel: 'MiniMax-M2.7',
    baseUrl: 'https://api.minimax.chat/v1',
  },
  {
    id: 'openai',
    name: 'OpenAI',
    defaultModel: 'gpt-4-turbo',
    baseUrl: 'https://api.openai.com/v1',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    defaultModel: 'deepseek-chat',
    baseUrl: 'https://api.deepseek.com/v1',
  },
  {
    id: 'zhipu',
    name: 'Zhipu (智谱)',
    defaultModel: 'glm-4',
    baseUrl: 'https://open.bigmodel.cn/api/paas/v1',
  },
  {
    id: 'ollama',
    name: 'Ollama (本地)',
    defaultModel: 'llama2',
    baseUrl: 'http://localhost:11434',
  },
];

function ClaudeSwitchSection() {
  const [backends, setBackends] = useState<ClaudeSwitchBackendInfo[]>([]);
  const [activeBackend, setActiveBackend] = useState<ClaudeSwitchBackendInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [addingBackend, setAddingBackend] = useState(false);
  const [testingBackend, setTestingBackend] = useState<string | null>(null);
  const [newBackend, setNewBackend] = useState<ClaudeSwitchBackendConfig>({
    backend: 'minimax',
    api_key: '',
    model: '',
  });

  const fetchBackends = async () => {
    setLoading(true);
    try {
      const list = await api.listClaudeSwitchBackends();
      setBackends(list);
      const active = await api.getActiveClaudeSwitchBackend();
      setActiveBackend(active);
    } catch (e) {
      console.error('Failed to fetch Claude Switch backends:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBackends();
  }, []);

  const handleAddBackend = async () => {
    if (!newBackend.api_key || !newBackend.model) {
      showError('请填写 API Key 和模型');
      return;
    }
    setAddingBackend(true);
    try {
      await api.addClaudeSwitchBackend(newBackend);
      showSuccess('后端添加成功');
      setNewBackend({ backend: 'minimax', api_key: '', model: '' });
      await fetchBackends();
    } catch (e) {
      showError('添加后端失败');
    } finally {
      setAddingBackend(false);
    }
  };

  const handleSwitchBackend = async (backend: string) => {
    try {
      await api.switchClaudeSwitchBackend(backend);
      showSuccess(`已切换到 ${backend}`);
      await fetchBackends();
    } catch (e) {
      showError('切换后端失败');
    }
  };

  const handleTestConnection = async () => {
    if (!newBackend.api_key || !newBackend.model) {
      showError('请填写 API Key 和模型');
      return;
    }
    setTestingBackend(newBackend.backend);
    try {
      const result = await api.testClaudeSwitchBackend(
        newBackend.backend,
        newBackend.api_key,
        newBackend.model,
      );
      if (result.success) {
        showSuccess('连接测试成功');
      } else {
        showError(`连接失败: ${result.message}`);
      }
    } catch (e) {
      showError('测试连接失败');
    } finally {
      setTestingBackend(null);
    }
  };

  const handleConfigure = async () => {
    if (backends.length === 0) {
      showError('请先添加后端');
      return;
    }
    try {
      await api.configureClaudeSwitch(
        backends.map((b) => ({
          backend: b.backend,
          api_key: '', // Will use existing
          model: b.model,
        })),
      );
      showSuccess('Claude Switch 配置成功');
    } catch (e) {
      showError('配置失败');
    }
  };

  const selectedBackendInfo = CLAUDE_SWITCH_BACKENDS.find((b) => b.id === newBackend.backend);

  return (
    <div className="bg-card rounded-xl border border-border/50 p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-500">
          <ArrowRightLeft className="w-5 h-5 text-white" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Claude Switch</h2>
          <p className="text-sm text-muted-foreground">用 Claude 接口，调用任意后端</p>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <>
          {/* Active Backend */}
          {activeBackend && (
            <div className="mb-6 p-4 rounded-lg bg-primary/5 border border-primary/20">
              <div className="flex items-center gap-2 mb-2">
                <Check className="w-4 h-4 text-primary" />
                <span className="text-sm font-medium text-primary">当前激活</span>
              </div>
              <p className="font-semibold">{activeBackend.backend.toUpperCase()}</p>
              <p className="text-sm text-muted-foreground">{activeBackend.model}</p>
            </div>
          )}

          {/* Backend List */}
          {backends.length > 0 && (
            <div className="mb-6">
              <p className="text-sm font-medium mb-3">已配置的后端</p>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                {backends.map((backend) => (
                  <button
                    key={backend.backend}
                    onClick={() => handleSwitchBackend(backend.backend)}
                    disabled={backend.is_active}
                    className={cn(
                      'p-3 rounded-lg border transition-all text-left',
                      backend.is_active
                        ? 'border-primary bg-primary/10 ring-1 ring-primary/50'
                        : 'border-border hover:border-primary/50 hover:bg-accent',
                    )}
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-medium">{backend.backend.toUpperCase()}</span>
                      {backend.is_active && <Check className="w-4 h-4 text-primary" />}
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">{backend.model}</p>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Add Backend Form */}
          <div className="border-t border-border/50 pt-6">
            <p className="text-sm font-medium mb-3">添加后端</p>
            <div className="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label className="text-xs text-muted-foreground block mb-1">后端类型</label>
                <select
                  value={newBackend.backend}
                  onChange={(e) =>
                    setNewBackend({
                      ...newBackend,
                      backend: e.target.value,
                      model:
                        CLAUDE_SWITCH_BACKENDS.find((b) => b.id === e.target.value)?.defaultModel ||
                        '',
                      base_url: CLAUDE_SWITCH_BACKENDS.find((b) => b.id === e.target.value)
                        ?.baseUrl,
                    })
                  }
                  className="w-full px-3 py-2 rounded-lg border border-border bg-background text-sm"
                >
                  {CLAUDE_SWITCH_BACKENDS.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.name}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-xs text-muted-foreground block mb-1">模型</label>
                <input
                  type="text"
                  value={newBackend.model}
                  onChange={(e) => setNewBackend({ ...newBackend, model: e.target.value })}
                  placeholder={selectedBackendInfo?.defaultModel}
                  className="w-full px-3 py-2 rounded-lg border border-border bg-background text-sm"
                />
              </div>
              <div className="col-span-2">
                <label className="text-xs text-muted-foreground block mb-1">API Key</label>
                <input
                  type="password"
                  value={newBackend.api_key}
                  onChange={(e) => setNewBackend({ ...newBackend, api_key: e.target.value })}
                  placeholder="输入 API Key"
                  className="w-full px-3 py-2 rounded-lg border border-border bg-background text-sm"
                />
              </div>
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleTestConnection}
                disabled={testingBackend !== null || !newBackend.api_key || !newBackend.model}
                className="btn-secondary flex-1"
              >
                {testingBackend !== null ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    测试中...
                  </>
                ) : (
                  '测试连接'
                )}
              </button>
              <button
                onClick={handleAddBackend}
                disabled={addingBackend || !newBackend.api_key || !newBackend.model}
                className="btn-primary flex-1"
              >
                {addingBackend ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    添加中...
                  </>
                ) : (
                  <>
                    <Plus className="w-4 h-4" />
                    添加后端
                  </>
                )}
              </button>
            </div>
          </div>

          {/* Configure Button */}
          {backends.length > 0 && (
            <button onClick={handleConfigure} className="btn-secondary w-full mt-4">
              保存 Claude Switch 配置
            </button>
          )}
        </>
      )}
    </div>
  );
}

export function AISettingsPage() {
  const {
    providersV2,
    presets,
    providersV2Loading,
    presetsLoading,
    mappings,
    mappingsLoading,
    fetchProvidersV2,
    fetchPresets,
    createProvider,
    updateProvider,
    deleteProvider,
    selectProvider,
    createFromPreset,
    fetchMappings,
    addMapping,
    removeMapping,
    testConnection,
    selectedProvider,
    enableProvider,
    disableProvider,
    selectedModel,
    fetchSelectedModel,
  } = useAIConfigStore();

  useEffect(() => {
    fetchProvidersV2();
    fetchPresets();
    fetchSelectedModel();
  }, [fetchProvidersV2, fetchPresets, fetchSelectedModel]);

  const currentMappings = selectedProvider ? mappings[selectedProvider.id] || [] : [];

  const handleAddMapping = async (
    providerId: string,
    mapping: { mapping_type: string; model_id: string; display_name?: string },
  ) => {
    await addMapping(providerId, mapping as AddModelMappingRequest);
  };

  return (
    <div className="page-container">
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight">AI 提供商</h1>
        <p className="text-sm text-muted-foreground mt-1">
          管理 AI 服务提供商，配置 API 密钥和模型映射
        </p>
      </div>

      {/* Claude CLI 路径配置（最重要，放最前） */}
      <div className="mb-6">
        <ClaudeCliPathSection />
      </div>

      {/* Claude Switch Section */}
      <div className="mb-6">
        <ClaudeSwitchSection />
      </div>

      {/* Provider Grid */}
      <ProviderGrid
        providers={providersV2}
        presets={presets}
        isLoading={providersV2Loading}
        isPresetsLoading={presetsLoading}
        mappings={currentMappings}
        mappingsLoading={mappingsLoading}
        onCreateProvider={createProvider}
        onUpdateProvider={updateProvider}
        onDeleteProvider={deleteProvider}
        onCreateFromPreset={createFromPreset}
        onSelectProvider={selectProvider}
        onFetchMappings={fetchMappings}
        onAddMapping={handleAddMapping}
        onRemoveMapping={removeMapping}
        onTestConnection={testConnection}
        onEnableProvider={enableProvider}
        onDisableProvider={disableProvider}
        selectedProvider={selectedProvider}
        selectedModel={selectedModel}
      />
    </div>
  );
}
