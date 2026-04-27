import { useState } from 'react';
import { Plus, Search, Loader2, Grid3X3, List, X, Wifi, WifiOff, Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  AIProvider,
  ProviderPreset,
  CreateProviderRequest,
  UpdateProviderRequest,
  ModelMapping,
  ConnectionTestResult,
  SelectedModelResponse,
} from '@/api/client';
import { ProviderCard } from './ProviderCard';
import { ProviderForm } from './ProviderForm';
import { ProviderPresetSelector } from './ProviderPresetSelector';
import { ModelMappingEditor } from './ModelMappingEditor';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';

interface ProviderGridProps {
  providers: AIProvider[];
  presets: ProviderPreset[];
  isLoading: boolean;
  isPresetsLoading: boolean;
  mappings: ModelMapping[];
  mappingsLoading: boolean;
  onCreateProvider: (data: CreateProviderRequest) => Promise<AIProvider | null>;
  onUpdateProvider: (id: string, data: UpdateProviderRequest) => Promise<AIProvider | null>;
  onDeleteProvider: (id: string) => Promise<boolean>;
  onCreateFromPreset: (presetKey: string, apiKey: string) => Promise<AIProvider | null>;
  onSelectProvider: (provider: AIProvider | null) => void;
  onFetchMappings: (providerId: string) => void;
  onAddMapping: (
    providerId: string,
    mapping: { mapping_type: string; model_id: string; display_name?: string },
  ) => Promise<void>;
  onRemoveMapping: (providerId: string, mappingId: string) => Promise<void>;
  onTestConnection: (providerId: string) => Promise<ConnectionTestResult>;
  onEnableProvider: (
    providerId: string,
    model?: string,
  ) => Promise<{ success: boolean; message: string; model: string } | null>;
  onDisableProvider: (
    providerId: string,
  ) => Promise<{ success: boolean; message: string; model: string } | null>;
  selectedProvider: AIProvider | null;
  selectedModel: SelectedModelResponse | null;
}

export function ProviderGrid({
  providers,
  presets,
  isLoading,
  isPresetsLoading,
  mappings,
  mappingsLoading,
  onCreateProvider,
  onUpdateProvider,
  onDeleteProvider,
  onCreateFromPreset,
  onSelectProvider,
  onFetchMappings,
  onAddMapping,
  onRemoveMapping,
  onTestConnection,
  onEnableProvider,
  onDisableProvider,
  selectedProvider,
  selectedModel,
}: ProviderGridProps) {
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();
  const [search, setSearch] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingProvider, setEditingProvider] = useState<AIProvider | null>(null);
  const [showPresetSelector, setShowPresetSelector] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<
    Record<string, { testing: boolean; result?: ConnectionTestResult }>
  >({});
  const [enablingProvider, setEnablingProvider] = useState<string | null>(null);

  const filteredProviders = providers.filter(
    (p) =>
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.provider_key.toLowerCase().includes(search.toLowerCase()) ||
      (p.description?.toLowerCase().includes(search.toLowerCase()) ?? false),
  );

  // Check if the selected provider is currently enabled
  const isProviderEnabled =
    selectedProvider && selectedModel
      ? selectedModel.model_id === `claude-switch-${selectedProvider.provider_key}`
      : false;

  const handleFormSubmit = async (data: CreateProviderRequest | UpdateProviderRequest) => {
    setIsSubmitting(true);
    try {
      if (editingProvider) {
        await onUpdateProvider(editingProvider.id, data as UpdateProviderRequest);
      } else {
        await onCreateProvider(data as CreateProviderRequest);
      }
      setShowForm(false);
      setEditingProvider(null);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteProvider = (id: string) => {
    showConfirm('删除提供商', '确定要删除这个提供商吗？此操作不可撤销。', () =>
      onDeleteProvider(id),
    );
  };

  const handleProviderSelect = (provider: AIProvider) => {
    onSelectProvider(provider);
    onFetchMappings(provider.id);
  };

  const handleTestConnection = async (providerId: string) => {
    setConnectionStatus((prev) => ({ ...prev, [providerId]: { testing: true } }));
    try {
      const result = await onTestConnection(providerId);
      setConnectionStatus((prev) => ({ ...prev, [providerId]: { testing: false, result } }));
    } catch (error) {
      setConnectionStatus((prev) => ({
        ...prev,
        [providerId]: { testing: false, result: { success: false, message: String(error) } },
      }));
    }
  };

  const handleEnableProvider = async (providerId: string) => {
    setEnablingProvider(providerId);
    try {
      if (isProviderEnabled) {
        // Currently enabled, so disable
        const result = await onDisableProvider(providerId);
        if (result?.success) {
          alert(`已关闭模型`);
        } else if (result) {
          alert(`关闭失败: ${result.message}`);
        }
      } else {
        // Currently disabled, so enable
        const result = await onEnableProvider(providerId);
        if (result?.success) {
          alert(`已启用模型: ${result.model}`);
        } else if (result) {
          alert(`启用失败: ${result.message}`);
        }
      }
    } catch (error) {
      alert(`操作失败: ${error}`);
    } finally {
      setEnablingProvider(null);
    }
  };

  return (
    <>
      <div className="space-y-4">
        {/* Toolbar */}
        <div className="flex items-center justify-between gap-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索提供商..."
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-input bg-background"
            />
          </div>

          <div className="flex items-center gap-2">
            {/* View Mode Toggle */}
            <div className="flex items-center border rounded-lg overflow-hidden">
              <button
                onClick={() => setViewMode('grid')}
                className={cn(
                  'p-2 transition-colors',
                  viewMode === 'grid' ? 'bg-primary text-primary-foreground' : 'hover:bg-accent',
                )}
              >
                <Grid3X3 className="w-4 h-4" />
              </button>
              <button
                onClick={() => setViewMode('list')}
                className={cn(
                  'p-2 transition-colors',
                  viewMode === 'list' ? 'bg-primary text-primary-foreground' : 'hover:bg-accent',
                )}
              >
                <List className="w-4 h-4" />
              </button>
            </div>

            <button
              onClick={() => setShowPresetSelector(true)}
              className="px-3 py-2 rounded-lg border border-input hover:bg-accent transition-colors flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              从预设添加
            </button>

            <button
              onClick={() => {
                setEditingProvider(null);
                setShowForm(true);
              }}
              className="px-3 py-2 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              自定义
            </button>
          </div>
        </div>

        {/* Content */}
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
          </div>
        ) : filteredProviders.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            {providers.length === 0 ? (
              <>
                <p className="mb-2">还没有配置任何提供商</p>
                <p className="text-sm">从预设添加或自定义一个新的提供商开始</p>
              </>
            ) : (
              <p>没有找到匹配的提供商</p>
            )}
          </div>
        ) : viewMode === 'grid' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredProviders.map((provider) => (
              <ProviderCard
                key={provider.id}
                provider={provider}
                isSelected={selectedProvider?.id === provider.id}
                onEdit={() => {
                  setEditingProvider(provider);
                  setShowForm(true);
                }}
                onDelete={() => handleDeleteProvider(provider.id)}
                onSelect={() => handleProviderSelect(provider)}
              />
            ))}
          </div>
        ) : (
          <div className="space-y-2">
            {filteredProviders.map((provider) => (
              <div
                key={provider.id}
                className={cn(
                  'p-3 rounded-lg border cursor-pointer transition-all',
                  selectedProvider?.id === provider.id
                    ? 'border-primary/50 bg-primary/5'
                    : 'border-border hover:border-primary/30',
                )}
                onClick={() => handleProviderSelect(provider)}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="font-medium">{provider.name}</span>
                    <span className="text-xs text-muted-foreground">{provider.provider_key}</span>
                    {provider.enabled ? (
                      <span className="px-2 py-0.5 text-xs bg-green-500/10 text-green-600 rounded-full">
                        启用
                      </span>
                    ) : (
                      <span className="px-2 py-0.5 text-xs bg-muted rounded text-muted-foreground">
                        禁用
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
                    <button
                      onClick={() => {
                        setEditingProvider(provider);
                        setShowForm(true);
                      }}
                      className="p-1.5 rounded-lg hover:bg-accent"
                    >
                      编辑
                    </button>
                    <button
                      onClick={() => handleDeleteProvider(provider.id)}
                      className="p-1.5 rounded-lg hover:bg-red-500/10 text-red-500"
                    >
                      删除
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Selected Provider Mapping Editor */}
        {selectedProvider && (
          <div className="mt-6 p-4 bg-card rounded-xl border border-border">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-medium">{selectedProvider.name} - 模型映射</h3>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleTestConnection(selectedProvider.id)}
                  disabled={connectionStatus[selectedProvider.id]?.testing}
                  className={cn(
                    'px-3 py-1 rounded-lg text-sm flex items-center gap-1 transition-colors',
                    connectionStatus[selectedProvider.id]?.result?.success === true
                      ? 'bg-green-500/10 text-green-600 border border-green-500/20'
                      : connectionStatus[selectedProvider.id]?.result?.success === false
                        ? 'bg-red-500/10 text-red-600 border border-red-500/20'
                        : 'bg-primary/10 text-primary hover:bg-primary/20',
                  )}
                >
                  {connectionStatus[selectedProvider.id]?.testing ? (
                    <Loader2 className="w-3 h-3 animate-spin" />
                  ) : connectionStatus[selectedProvider.id]?.result?.success === true ? (
                    <Wifi className="w-3 h-3" />
                  ) : connectionStatus[selectedProvider.id]?.result?.success === false ? (
                    <WifiOff className="w-3 h-3" />
                  ) : (
                    <Wifi className="w-3 h-3" />
                  )}
                  测试连接
                </button>
                <button
                  onClick={() => handleEnableProvider(selectedProvider.id)}
                  disabled={enablingProvider === selectedProvider.id}
                  className={cn(
                    'px-3 py-1 rounded-lg text-sm flex items-center gap-1 transition-colors',
                    isProviderEnabled
                      ? 'bg-red-500/10 text-red-600 border border-red-500/20 hover:bg-red-500/20'
                      : 'bg-green-500/10 text-green-600 border border-green-500/20 hover:bg-green-500/20',
                  )}
                >
                  {enablingProvider === selectedProvider.id ? (
                    <Loader2 className="w-3 h-3 animate-spin" />
                  ) : isProviderEnabled ? (
                    <X className="w-3 h-3" />
                  ) : (
                    <Check className="w-3 h-3" />
                  )}
                  {isProviderEnabled ? '关闭模型' : '启用模型'}
                </button>
                <button
                  onClick={() => onSelectProvider(null)}
                  className="p-1 rounded-lg hover:bg-accent"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
            {connectionStatus[selectedProvider.id]?.result && (
              <div
                className={cn(
                  'mb-4 p-3 rounded-lg text-sm',
                  connectionStatus[selectedProvider.id].result!.success
                    ? 'bg-green-500/10 text-green-600'
                    : 'bg-red-500/10 text-red-600',
                )}
              >
                <p className="font-medium">
                  {connectionStatus[selectedProvider.id].result!.message}
                </p>
                {connectionStatus[selectedProvider.id].result!.models && (
                  <p className="mt-1 text-xs opacity-80">
                    可用模型:{' '}
                    {connectionStatus[selectedProvider.id].result!.models?.slice(0, 5).join(', ')}
                    {connectionStatus[selectedProvider.id].result!.models!.length > 5 && '...'}
                  </p>
                )}
              </div>
            )}
            <ModelMappingEditor
              provider={selectedProvider}
              mappings={mappings}
              isLoading={mappingsLoading}
              onAddMapping={async (providerId, mapping) => {
                await onAddMapping(providerId, mapping);
              }}
              onRemoveMapping={onRemoveMapping}
            />
          </div>
        )}

        {/* Form Modal */}
        {showForm && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div className="bg-background rounded-xl shadow-xl w-full max-w-lg max-h-[90vh] overflow-y-auto p-6">
              <ProviderForm
                provider={editingProvider}
                onSubmit={handleFormSubmit}
                onCancel={() => {
                  setShowForm(false);
                  setEditingProvider(null);
                }}
                isLoading={isSubmitting}
              />
            </div>
          </div>
        )}

        {/* Preset Selector Modal */}
        {showPresetSelector && (
          <ProviderPresetSelector
            presets={presets}
            isLoading={isPresetsLoading}
            onSelect={async (preset, apiKey) => {
              await onCreateFromPreset(preset.key, apiKey);
            }}
            onClose={() => setShowPresetSelector(false)}
          />
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
