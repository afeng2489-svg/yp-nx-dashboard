import { useEffect, useState } from 'react';
import { Moon, Sun } from 'lucide-react';
import { THEMES, DEFAULT_THEME, type ThemeMode } from '@/themes/registry';
import { cn } from '@/lib/cn';

/** 演示用：运行时切换主题 + 明暗。成品站通常固定单主题，不需要这个控件。 */
export function ThemeControls({ className }: { className?: string }) {
  const [theme, setTheme] = useState(DEFAULT_THEME.id);
  const [mode, setMode] = useState<ThemeMode>(DEFAULT_THEME.defaultMode);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = theme;
    root.classList.toggle('dark', mode === 'dark');
  }, [theme, mode]);

  const onPickTheme = (id: string) => {
    setTheme(id);
    const meta = THEMES.find((t) => t.id === id);
    if (meta) setMode(meta.defaultMode);
  };

  return (
    <div className={cn('flex items-center gap-2', className)}>
      <select
        aria-label="选择主题"
        value={theme}
        onChange={(e) => onPickTheme(e.target.value)}
        className="h-9 rounded-lg border border-border bg-surface px-3 text-sm font-medium text-ink outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
      >
        {THEMES.map((t) => (
          <option key={t.id} value={t.id}>
            {t.name}
          </option>
        ))}
      </select>
      <button
        type="button"
        aria-label="切换明暗"
        onClick={() => setMode((m) => (m === 'dark' ? 'light' : 'dark'))}
        className="grid h-9 w-9 place-items-center rounded-lg border border-border bg-surface text-ink transition-colors hover:bg-ink/5"
      >
        {mode === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
      </button>
    </div>
  );
}
