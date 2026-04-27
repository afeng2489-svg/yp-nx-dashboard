import { Edit, Trash2, Globe, Key, CheckCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { AIProvider } from '@/api/client';

interface ProviderCardProps {
  provider: AIProvider;
  onEdit: () => void;
  onDelete: () => void;
  onSelect: () => void;
  isSelected?: boolean;
}

export function ProviderCard({
  provider,
  onEdit,
  onDelete,
  onSelect,
  isSelected,
}: ProviderCardProps) {
  const getFormatLabel = (format: AIProvider['api_format']) => {
    if (typeof format === 'string') {
      return format === 'openai' ? 'OpenAI' : format === 'anthropic' ? 'Anthropic' : format;
    }
    return `Custom: ${format.custom}`;
  };

  return (
    <div
      className={cn(
        'bg-gradient-to-br from-card to-muted/20 rounded-xl p-4 border transition-all duration-200 cursor-pointer',
        isSelected
          ? 'border-primary/50 shadow-lg shadow-primary/10'
          : 'border-border hover:border-primary/30 hover:shadow-md',
      )}
      onClick={onSelect}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Globe className="w-4 h-4 text-primary" />
            <span className="font-semibold truncate">{provider.name}</span>
            {provider.enabled ? (
              <CheckCircle className="w-3.5 h-3.5 text-green-500" />
            ) : (
              <span className="px-2 py-0.5 text-xs bg-muted rounded text-muted-foreground">
                禁用
              </span>
            )}
          </div>

          {provider.description && (
            <p className="text-xs text-muted-foreground/80 line-clamp-2 mb-2">
              {provider.description}
            </p>
          )}

          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <span className="px-2 py-0.5 bg-muted rounded border">
              {getFormatLabel(provider.api_format)}
            </span>
            <span className="flex items-center gap-1">
              <Key className="w-3 h-3" />
              API Key
            </span>
          </div>

          {provider.website && (
            <a
              href={provider.website}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary hover:underline mt-1 block truncate"
              onClick={(e) => e.stopPropagation()}
            >
              {provider.website}
            </a>
          )}
        </div>

        <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={onEdit}
            className={cn(
              'p-2 rounded-lg transition-all duration-200',
              'hover:bg-indigo-500/10 text-muted-foreground hover:text-indigo-500',
            )}
            title="编辑"
          >
            <Edit className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onDelete}
            className={cn(
              'p-2 rounded-lg transition-all duration-200',
              'hover:bg-red-500/10 text-muted-foreground hover:text-red-500',
            )}
            title="删除"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
