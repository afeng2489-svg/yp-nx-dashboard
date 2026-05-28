import { useState } from 'react';
import { ChevronDown, Layers } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { DEFAULT_LAYOUT_VARIANT, LAYOUT_VARIANTS, type LayoutVariant } from '@/data/layoutVariants';
import { layoutMenuItemClassName, layoutMenuPanelClassName } from '@/components/layout/layoutMenuStyles';

interface LayoutVariantMenuProps {
  compact?: boolean;
}

/** 经典 / 精炼界面切换 */
export function LayoutVariantMenu({ compact = false }: LayoutVariantMenuProps) {
  const [open, setOpen] = useState(false);
  const variant = useSettingsStore((s) => s.layout.variant ?? DEFAULT_LAYOUT_VARIANT);
  const setLayout = useSettingsStore((s) => s.setLayout);
  const current = LAYOUT_VARIANTS.find((v) => v.id === variant) ?? LAYOUT_VARIANTS[0];

  const pick = (next: LayoutVariant) => {
    setLayout({ variant: next });
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
        title="经典 / 精炼界面"
      >
        <Layers className="w-3.5 h-3.5 text-muted-foreground" />
        <span className="text-xs font-medium">{compact ? (variant === 'classic' ? '经典' : '精炼') : current.label}</span>
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
          <div className={layoutMenuPanelClassName('sm')}>
            {LAYOUT_VARIANTS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                onClick={() => pick(opt.id)}
                className={layoutMenuItemClassName(variant === opt.id)}
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
