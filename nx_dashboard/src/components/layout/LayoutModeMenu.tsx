import { useState } from 'react';
import { ChevronDown, LayoutTemplate } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { LAYOUT_MODES, type LayoutMode } from '@/data/layoutModes';
import { layoutMenuItemClassName, layoutMenuPanelClassName } from '@/components/layout/layoutMenuStyles';

interface LayoutModeMenuProps {
  compact?: boolean;
}

/** 布局模式切换（引导 / 工作室） */
export function LayoutModeMenu({ compact = false }: LayoutModeMenuProps) {
  const [open, setOpen] = useState(false);
  const mode = useSettingsStore((s) => s.layout.mode);
  const setLayout = useSettingsStore((s) => s.setLayout);
  const current = LAYOUT_MODES.find((m) => m.id === mode) ?? LAYOUT_MODES[0];

  const pick = (next: LayoutMode) => {
    setLayout({ mode: next });
    setOpen(false);
  };

  return (
    <div className="relative shrink-0">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={cn(
          'inline-flex items-center gap-1.5 rounded-lg border border-border/50 hover:bg-accent/50 transition-colors text-sm',
          compact ? 'px-2 py-1' : 'px-2.5 py-1.5',
        )}
        title="切换布局模式"
      >
        <LayoutTemplate className="w-3.5 h-3.5 text-muted-foreground" />
        <span className="text-xs font-medium">{current.shortLabel}</span>
        <ChevronDown className="w-3 h-3 text-muted-foreground" />
      </button>
      {open && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 cursor-default"
            aria-label="关闭菜单"
            onClick={() => setOpen(false)}
          />
          <div className={layoutMenuPanelClassName('md')}>
            {LAYOUT_MODES.map((opt) => (
              <button
                key={opt.id}
                type="button"
                onClick={() => pick(opt.id)}
                className={layoutMenuItemClassName(mode === opt.id)}
              >
                <p className="text-sm font-medium">{opt.label}</p>
                <p className="text-[11px] text-muted-foreground mt-0.5">{opt.description}</p>
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
