import { FolderOpen, Globe } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { InputMode } from '../types';

// ── 模式切换按钮 ──────────────────────────────────────
export function ModeSwitcher({
  mode,
  onChange,
}: {
  mode: InputMode;
  onChange: (m: InputMode) => void;
}) {
  return (
    <div className="flex items-center gap-1 p-1 bg-muted/50 rounded-xl w-fit">
      <button
        onClick={() => onChange('file')}
        className={cn(
          'flex items-center gap-2 px-4 py-1.5 rounded-lg text-sm font-medium transition-all',
          mode === 'file'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        )}
      >
        <FolderOpen className="w-4 h-4" />
        文件模式
      </button>
      <button
        onClick={() => onChange('url')}
        className={cn(
          'flex items-center gap-2 px-4 py-1.5 rounded-lg text-sm font-medium transition-all',
          mode === 'url'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        )}
      >
        <Globe className="w-4 h-4" />
        URL 模式
      </button>
    </div>
  );
}
