import { useState } from 'react';
import { X, Search, Loader2, CheckCircle, ExternalLink } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ProviderPreset, APIFormat } from '@/api/client';

interface ProviderPresetSelectorProps {
  presets: ProviderPreset[];
  isLoading: boolean;
  onSelect: (preset: ProviderPreset, apiKey: string) => Promise<void>;
  onClose: () => void;
}

export function ProviderPresetSelector({
  presets,
  isLoading,
  onSelect,
  onClose,
}: ProviderPresetSelectorProps) {
  const [search, setSearch] = useState('');
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [apiKeyError, setApiKeyError] = useState('');
  const [isConfirming, setIsConfirming] = useState(false);

  const filteredPresets = presets.filter(
    (preset) =>
      preset.name.toLowerCase().includes(search.toLowerCase()) ||
      preset.description.toLowerCase().includes(search.toLowerCase()) ||
      preset.key.toLowerCase().includes(search.toLowerCase()),
  );

  const getFormatLabel = (format: APIFormat) => {
    if (typeof format === 'string') {
      return format === 'openai' ? 'OpenAI' : format === 'anthropic' ? 'Anthropic' : format;
    }
    return `Custom: ${format.custom}`;
  };

  const handleConfirm = async () => {
    if (!selectedPreset) return;
    if (!apiKey.trim()) {
      setApiKeyError('API Key 不能为空');
      return;
    }
    setIsConfirming(true);
    try {
      await onSelect(selectedPreset, apiKey);
    } finally {
      setIsConfirming(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold">选择预设提供商</h2>
          <button onClick={onClose} className="p-1 rounded-lg hover:bg-accent">
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Search */}
        <div className="p-4 border-b">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索提供商..."
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-input bg-background"
            />
          </div>
        </div>

        {/* Preset List */}
        <div className="flex-1 overflow-y-auto p-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {filteredPresets.map((preset) => (
                <div
                  key={preset.key}
                  className={cn(
                    'p-3 rounded-lg border cursor-pointer transition-all',
                    selectedPreset?.key === preset.key
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:border-primary/30',
                  )}
                  onClick={() => setSelectedPreset(preset)}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium truncate">{preset.name}</span>
                        {selectedPreset?.key === preset.key && (
                          <CheckCircle className="w-4 h-4 text-primary flex-shrink-0" />
                        )}
                      </div>
                      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">
                        {preset.description}
                      </p>
                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-xs px-1.5 py-0.5 bg-muted rounded">
                          {getFormatLabel(preset.api_format)}
                        </span>
                      </div>
                    </div>
                    <a
                      href={preset.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="p-1 rounded hover:bg-accent flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <ExternalLink className="w-3 h-3 text-muted-foreground" />
                    </a>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* API Key Input & Confirm */}
        {selectedPreset && (
          <div className="p-4 border-t space-y-3">
            <div>
              <label className="block text-sm font-medium mb-1">
                {selectedPreset.name} API Key <span className="text-red-500">*</span>
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => {
                  setApiKey(e.target.value);
                  setApiKeyError('');
                }}
                placeholder="输入 API Key"
                className={cn(
                  'w-full px-3 py-2 rounded-lg border bg-background',
                  apiKeyError ? 'border-red-500' : 'border-input',
                )}
              />
              {apiKeyError && <p className="text-xs text-red-500 mt-1">{apiKeyError}</p>}
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={onClose}
                className="px-4 py-2 rounded-lg border border-input hover:bg-accent transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleConfirm}
                disabled={!apiKey.trim() || isConfirming}
                className={cn(
                  'px-4 py-2 rounded-lg bg-primary text-primary-foreground transition-colors',
                  'hover:bg-primary/90 disabled:opacity-50 flex items-center gap-2',
                )}
              >
                {isConfirming ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <CheckCircle className="w-4 h-4" />
                )}
                添加 {selectedPreset.name}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
