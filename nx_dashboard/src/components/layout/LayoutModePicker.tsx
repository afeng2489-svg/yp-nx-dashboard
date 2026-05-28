import { cn } from '@/lib/utils';
import { LAYOUT_MODES, type LayoutMode } from '@/data/layoutModes';
import { useSettingsStore } from '@/stores/settingsStore';

/** 设置页 — 三模式布局选择 */
export function LayoutModePicker() {
  const mode = useSettingsStore((s) => s.layout.mode);
  const setLayout = useSettingsStore((s) => s.setLayout);

  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm font-medium">布局模式</p>
        <p className="text-xs text-muted-foreground mt-0.5">
          同一套功能，两种界面 — 可随时切换
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {LAYOUT_MODES.map((opt) => (
          <button
            key={opt.id}
            type="button"
            onClick={() => setLayout({ mode: opt.id as LayoutMode })}
            className={cn(
              'flex flex-col items-start gap-1 p-4 rounded-xl border text-left transition-colors',
              mode === opt.id
                ? 'border-primary bg-primary/5 shadow-sm'
                : 'border-border hover:border-primary/40',
            )}
          >
            <span className={cn('text-sm font-medium', mode === opt.id && 'text-primary')}>
              {opt.label}
            </span>
            <span className="text-[11px] text-muted-foreground">{opt.persona}</span>
            <span className="text-xs text-muted-foreground leading-snug">{opt.description}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
