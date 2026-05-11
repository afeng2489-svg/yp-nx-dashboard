import type { FieldDef } from '../types';

// ── 通用输入表单 ──────────────────────────────────────
export function FieldForm({
  fields,
  values,
  onChange,
}: {
  fields: FieldDef[];
  values: Record<string, string>;
  onChange: (k: string, v: string) => void;
}) {
  return (
    <div className="space-y-4">
      {fields.map((f) => (
        <div key={f.key}>
          <label className="block text-sm font-medium mb-1">
            {f.label}
            {f.required && <span className="text-red-500 ml-1">*</span>}
          </label>
          <p className="text-xs text-muted-foreground mb-1.5">{f.desc}</p>
          {f.multiline ? (
            <textarea
              className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none min-h-[100px] font-mono text-xs"
              placeholder={f.desc}
              value={values[f.key] ?? ''}
              onChange={(e) => onChange(f.key, e.target.value)}
            />
          ) : (
            <input
              type="text"
              className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
              placeholder={f.desc}
              value={values[f.key] ?? ''}
              onChange={(e) => onChange(f.key, e.target.value)}
            />
          )}
        </div>
      ))}
    </div>
  );
}
