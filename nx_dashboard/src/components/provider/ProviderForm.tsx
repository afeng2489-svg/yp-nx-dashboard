import { useState, useEffect } from 'react';
import { X, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { AIProvider, CreateProviderRequest, UpdateProviderRequest, APIFormat } from '@/api/client';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

interface ProviderFormProps {
  provider?: AIProvider | null;
  onSubmit: (data: CreateProviderRequest | UpdateProviderRequest) => Promise<void>;
  onCancel: () => void;
  isLoading?: boolean;
}

export function ProviderForm({ provider, onSubmit, onCancel, isLoading }: ProviderFormProps) {
  const [formData, setFormData] = useState({
    name: '',
    provider_key: '',
    description: '',
    website: '',
    base_url: '',
    api_format: 'openai' as APIFormat,
    auth_field: 'Authorization',
    api_key: '',
    enabled: true,
    config_json: '',
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (provider) {
      setFormData({
        name: provider.name,
        provider_key: provider.provider_key,
        description: provider.description || '',
        website: provider.website || '',
        base_url: provider.base_url,
        api_format: provider.api_format,
        auth_field: provider.auth_field,
        api_key: '', // Don't prefill API key for security
        enabled: provider.enabled,
        config_json: provider.config_json || '',
      });
    }
  }, [provider]);

  const validate = () => {
    const newErrors: Record<string, string> = {};
    if (!formData.name.trim()) newErrors.name = '名称不能为空';
    if (!formData.provider_key.trim()) newErrors.provider_key = '标识不能为空';
    if (!formData.base_url.trim()) newErrors.base_url = '请求地址不能为空';
    if (!provider && !formData.api_key.trim()) newErrors.api_key = 'API Key不能为空';
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    // Only include api_key if it's been changed/entered
    const data = provider
      ? {
          name: formData.name,
          provider_key: formData.provider_key,
          description: formData.description || undefined,
          website: formData.website || undefined,
          base_url: formData.base_url,
          api_format: formData.api_format,
          auth_field: formData.auth_field,
          enabled: formData.enabled,
          config_json: formData.config_json || undefined,
          api_key: formData.api_key || undefined,
        }
      : formData;

    await onSubmit(data);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">{provider ? '编辑提供商' : '新建提供商'}</h3>
        <button onClick={onCancel} className="p-1 rounded-lg hover:bg-accent">
          <X className="w-4 h-4" />
        </button>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Name */}
        <div>
          <label className="block text-sm font-medium mb-1">
            供应商名称 <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            className={cn(
              'w-full px-3 py-2 rounded-lg border bg-background',
              errors.name ? 'border-red-500' : 'border-input',
            )}
            placeholder="例如：DeepSeek"
          />
          {errors.name && <p className="text-xs text-red-500 mt-1">{errors.name}</p>}
        </div>

        {/* Provider Key */}
        <div>
          <label className="block text-sm font-medium mb-1">
            标识 <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={formData.provider_key}
            onChange={(e) => setFormData({ ...formData, provider_key: e.target.value })}
            className={cn(
              'w-full px-3 py-2 rounded-lg border bg-background',
              errors.provider_key ? 'border-red-500' : 'border-input',
            )}
            placeholder="例如：deepseek"
          />
          {errors.provider_key && (
            <p className="text-xs text-red-500 mt-1">{errors.provider_key}</p>
          )}
        </div>

        {/* Description */}
        <div>
          <label className="block text-sm font-medium mb-1">备注</label>
          <input
            type="text"
            value={formData.description}
            onChange={(e) => setFormData({ ...formData, description: e.target.value })}
            className="w-full px-3 py-2 rounded-lg border border-input bg-background"
            placeholder="可选描述"
          />
        </div>

        {/* Website */}
        <div>
          <label className="block text-sm font-medium mb-1">官网链接</label>
          <input
            type="url"
            value={formData.website}
            onChange={(e) => setFormData({ ...formData, website: e.target.value })}
            className="w-full px-3 py-2 rounded-lg border border-input bg-background"
            placeholder="https://..."
          />
        </div>

        {/* Base URL */}
        <div>
          <label className="block text-sm font-medium mb-1">
            请求地址 <span className="text-red-500">*</span>
          </label>
          <input
            type="url"
            value={formData.base_url}
            onChange={(e) => setFormData({ ...formData, base_url: e.target.value })}
            className={cn(
              'w-full px-3 py-2 rounded-lg border bg-background',
              errors.base_url ? 'border-red-500' : 'border-input',
            )}
            placeholder="https://api.example.com/v1/chat/completions"
          />
          {errors.base_url && <p className="text-xs text-red-500 mt-1">{errors.base_url}</p>}
        </div>

        {/* API Format */}
        <div>
          <label className="block text-sm font-medium mb-1">API 格式</label>
          <Select
            value={typeof formData.api_format === 'string' ? formData.api_format : 'openai'}
            onValueChange={(v) => setFormData({ ...formData, api_format: v as APIFormat })}
          >
            <SelectTrigger className="h-8 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="openai">OpenAI Compatible</SelectItem>
              <SelectItem value="anthropic">Anthropic Message (原生)</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Auth Field */}
        <div>
          <label className="block text-sm font-medium mb-1">认证字段</label>
          <Select
            value={formData.auth_field}
            onValueChange={(v) => setFormData({ ...formData, auth_field: v })}
          >
            <SelectTrigger className="h-8 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="Authorization">Authorization (默认)</SelectItem>
              <SelectItem value="x-api-key">x-api-key</SelectItem>
              <SelectItem value="api-key">api-key</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* API Key */}
        <div>
          <label className="block text-sm font-medium mb-1">
            {provider ? '新 API Key (留空则不修改)' : 'API Key'}
            {!provider && <span className="text-red-500">*</span>}
          </label>
          <input
            type="password"
            value={formData.api_key}
            onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
            className={cn(
              'w-full px-3 py-2 rounded-lg border bg-background',
              errors.api_key ? 'border-red-500' : 'border-input',
            )}
            placeholder={provider ? '留空保持不变' : '输入 API Key'}
          />
          {errors.api_key && <p className="text-xs text-red-500 mt-1">{errors.api_key}</p>}
        </div>

        {/* Enabled */}
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="enabled"
            checked={formData.enabled}
            onChange={(e) => setFormData({ ...formData, enabled: e.target.checked })}
            className="w-4 h-4 rounded border-input text-primary focus:ring-primary"
          />
          <label htmlFor="enabled" className="text-sm">
            启用此提供商
          </label>
        </div>

        {/* Config JSON */}
        <div>
          <label className="block text-sm font-medium mb-1">配置 JSON</label>
          <textarea
            value={formData.config_json}
            onChange={(e) => setFormData({ ...formData, config_json: e.target.value })}
            className="w-full px-3 py-2 rounded-lg border border-input bg-background font-mono text-sm"
            rows={4}
            placeholder='{"model_mappings": {...}}'
          />
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 rounded-lg border border-input hover:bg-accent transition-colors"
          >
            取消
          </button>
          <button
            type="submit"
            disabled={isLoading}
            className={cn(
              'px-4 py-2 rounded-lg bg-primary text-primary-foreground transition-colors',
              'hover:bg-primary/90 disabled:opacity-50 flex items-center gap-2',
            )}
          >
            {isLoading && <Loader2 className="w-4 h-4 animate-spin" />}
            {provider ? '保存更改' : '创建提供商'}
          </button>
        </div>
      </form>
    </div>
  );
}
