import { Moon, Sun, Monitor } from 'lucide-react';
import { useThemeStore, Theme } from '@/stores/themeStore';
import { cn } from '@/lib/utils';

const themes: { value: Theme; icon: React.ComponentType<{ className?: string }>; label: string }[] =
  [
    { value: 'light', icon: Sun, label: '浅色' },
    { value: 'dark', icon: Moon, label: '深色' },
    { value: 'system', icon: Monitor, label: '系统' },
  ];

export function ThemeToggle({ className }: { className?: string }) {
  const { theme, setTheme } = useThemeStore();

  const cycleTheme = () => {
    const currentIndex = themes.findIndex((t) => t.value === theme);
    const nextIndex = (currentIndex + 1) % themes.length;
    setTheme(themes[nextIndex].value);
  };

  const CurrentIcon = themes.find((t) => t.value === theme)!.icon;

  return (
    <div className={cn('relative flex items-center gap-1 p-1 bg-accent rounded-lg', className)}>
      <button
        onClick={cycleTheme}
        className={cn(
          'flex items-center gap-2 px-2 py-1.5 rounded-md text-sm font-medium transition-all duration-200',
          'hover:bg-background hover:text-foreground',
        )}
        title={`当前: ${themes.find((t) => t.value === theme)?.label}`}
      >
        <CurrentIcon className="w-4 h-4" />
        <span className="hidden sm:inline">{themes.find((t) => t.value === theme)?.label}</span>
      </button>
    </div>
  );
}
