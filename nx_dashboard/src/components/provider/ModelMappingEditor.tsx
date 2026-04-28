import { useState } from 'react';
import { Plus, Trash2, Loader2, Brain, Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import { AIProvider, ModelMapping, MappingType, AddModelMappingRequest } from '@/api/client';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';

interface ModelMappingEditorProps {
  provider: AIProvider;
  mappings: ModelMapping[];
  isLoading: boolean;
  onAddMapping: (providerId: string, mapping: AddModelMappingRequest) => Promise<void>;
  onRemoveMapping: (providerId: string, mappingId: string) => Promise<void>;
}

const MAPPING_TYPE_LABELS: Record<
  MappingType,
  { label: string; icon: React.ReactNode; color: string }
> = {
  main: {
    label: '主模型',
    icon: <Sparkles className="w-3 h-3" />,
    color: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  },
  thinking: {
    label: '推理模型 (Thinking)',
    icon: <Brain className="w-3 h-3" />,
    color: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  },
  haiku: {
    label: 'Haiku 默认',
    icon: <Sparkles className="w-3 h-3" />,
    color: 'bg-green-500/10 text-green-600 border-green-500/20',
  },
  sonnet: {
    label: 'Sonnet 默认',
    icon: <Sparkles className="w-3 h-3" />,
    color: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  },
  opus: {
    label: 'Opus 默认',
    icon: <Sparkles className="w-3 h-3" />,
    color: 'bg-red-500/10 text-red-600 border-red-500/20',
  },
};

export function ModelMappingEditor({
  provider,
  mappings,
  isLoading,
  onAddMapping,
  onRemoveMapping,
}: ModelMappingEditorProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [newMapping, setNewMapping] = useState<AddModelMappingRequest>({
    mapping_type: 'main',
    model_id: '',
    display_name: '',
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const handleAdd = async () => {
    if (!newMapping.model_id.trim()) return;
    setIsSubmitting(true);
    setError(null);
    try {
      await onAddMapping(provider.id, newMapping);
      setShowAddForm(false);
      setNewMapping({ mapping_type: 'main', model_id: '', display_name: '' });
    } catch (err) {
      console.error('[ModelMappingEditor] error:', err);
      setError(err instanceof Error ? err.message : '添加映射失败');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleRemove = (mappingId: string) => {
    showConfirm('删除模型映射', '确定要删除这个模型映射吗？', () =>
      onRemoveMapping(provider.id, mappingId),
    );
  };

  const mappingsByType = mappings.reduce(
    (acc, m) => {
      if (!acc[m.mapping_type]) acc[m.mapping_type] = [];
      acc[m.mapping_type].push(m);
      return acc;
    },
    {} as Record<MappingType, ModelMapping[]>,
  );

  return (
    <>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h4 className="font-medium">模型映射</h4>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className={cn(
              'px-2 py-1 rounded-lg text-sm transition-colors',
              showAddForm
                ? 'bg-destructive/10 text-destructive'
                : 'bg-primary/10 text-primary hover:bg-primary/20',
            )}
          >
            {showAddForm ? (
              '取消'
            ) : (
              <>
                <Plus className="w-3 h-3 inline mr-1" />
                添加映射
              </>
            )}
          </button>
        </div>

        {/* Add Form */}
        {showAddForm && (
          <div className="p-3 bg-muted/50 rounded-lg space-y-3">
            <div>
              <label className="block text-xs font-medium mb-1">映射类型</label>
              <select
                value={newMapping.mapping_type}
                onChange={(e) =>
                  setNewMapping({ ...newMapping, mapping_type: e.target.value as MappingType })
                }
                className="w-full px-2 py-1.5 rounded border border-input bg-background text-sm"
              >
                <option value="main">主模型</option>
                <option value="thinking">推理模型 (Thinking)</option>
                <option value="haiku">Haiku 默认</option>
                <option value="sonnet">Sonnet 默认</option>
                <option value="opus">Opus 默认</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium mb-1">模型 ID</label>
              <input
                type="text"
                value={newMapping.model_id}
                onChange={(e) => setNewMapping({ ...newMapping, model_id: e.target.value })}
                placeholder="例如：deepseek-chat"
                className="w-full px-2 py-1.5 rounded border border-input bg-background text-sm"
              />
            </div>

            <div>
              <label className="block text-xs font-medium mb-1">显示名称 (可选)</label>
              <input
                type="text"
                value={newMapping.display_name}
                onChange={(e) => setNewMapping({ ...newMapping, display_name: e.target.value })}
                placeholder="例如：DeepSeek Chat"
                className="w-full px-2 py-1.5 rounded border border-input bg-background text-sm"
              />
            </div>

            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowAddForm(false)}
                className="px-3 py-1 rounded border border-input hover:bg-accent text-sm"
              >
                取消
              </button>
              <button
                onClick={handleAdd}
                disabled={!newMapping.model_id.trim() || isSubmitting}
                className="px-3 py-1 rounded bg-primary text-primary-foreground text-sm hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1"
              >
                {isSubmitting && <Loader2 className="w-3 h-3 animate-spin" />}
                添加
              </button>
            </div>
            {error && (
              <div className="mt-2 p-2 bg-destructive/10 text-destructive text-sm rounded">
                {error}
              </div>
            )}
          </div>
        )}

        {/* Mappings List */}
        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
          </div>
        ) : mappings.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">还没有配置模型映射</p>
        ) : (
          <div className="space-y-3">
            {(Object.keys(MAPPING_TYPE_LABELS) as MappingType[]).map((type) => {
              const typeMappings = mappingsByType[type];
              if (!typeMappings || typeMappings.length === 0) return null;

              const typeInfo = MAPPING_TYPE_LABELS[type];

              return (
                <div key={type} className="space-y-2">
                  <div
                    className={cn(
                      'inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs border',
                      typeInfo.color,
                    )}
                  >
                    {typeInfo.icon}
                    {typeInfo.label}
                  </div>
                  <div className="space-y-1">
                    {typeMappings.map((mapping) => (
                      <div
                        key={mapping.id}
                        className="flex items-center justify-between p-2 bg-card rounded border border-border"
                      >
                        <div>
                          <span className="font-mono text-sm">{mapping.model_id}</span>
                          {mapping.display_name && (
                            <span className="text-xs text-muted-foreground ml-2">
                              ({mapping.display_name})
                            </span>
                          )}
                        </div>
                        <button
                          onClick={() => handleRemove(mapping.id)}
                          className="p-1 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500"
                        >
                          <Trash2 className="w-3 h-3" />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })}
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
