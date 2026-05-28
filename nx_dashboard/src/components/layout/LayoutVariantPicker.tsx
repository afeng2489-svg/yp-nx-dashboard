import { cn } from '@/lib/utils';
import { DEFAULT_LAYOUT_VARIANT, LAYOUT_VARIANTS, type LayoutVariant } from '@/data/layoutVariants';
import { useSettingsStore } from '@/stores/settingsStore';

/** 设置页 — 经典 / 精炼界面 */
export function LayoutVariantPicker() {
  const variant = useSettingsStore((s) => s.layout.variant ?? DEFAULT_LAYOUT_VARIANT);
  const setLayout = useSettingsStore((s) => s.setLayout);

  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm font-medium">界面风格</p>
        <p className="text-xs text-muted-foreground mt-0.5">
          精炼为默认推荐；经典版保留 AF-10 原版（可随时切回）
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {LAYOUT_VARIANTS.map((opt) => (
          <button
            key={opt.id}
            type="button"
            onClick={() => setLayout({ variant: opt.id as LayoutVariant })}
            className={cn(
              'flex flex-col items-start gap-1 p-4 rounded-xl border text-left transition-colors',
              variant === opt.id
                ? 'border-primary bg-primary/5 shadow-sm'
                : 'border-border hover:border-primary/40',
            )}
          >
            <span className={cn('text-sm font-medium', variant === opt.id && 'text-primary')}>
              {opt.label}
            </span>
            <span className="text-xs text-muted-foreground leading-snug">{opt.description}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
