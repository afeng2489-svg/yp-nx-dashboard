import { Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { FormField, FormSection, formControlClass, formTextareaClass } from '@/components/ui/formStyles';
import {
  fieldLabel,
  fromSelectValue,
  sortFieldEntries,
  toSelectValue,
} from './launchFormUtils';

export interface InputChoice {
  value: string;
  label?: string;
  description?: string;
}

export interface LaunchFormInput {
  type: string;
  required?: boolean;
  description?: string;
  options?: InputChoice[];
  examples?: InputChoice[];
}

interface LaunchFormFieldsProps {
  inputs: Record<string, LaunchFormInput>;
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
}

const CORE_KEYS = new Set(['brand', 'description']);
const STYLE_KEYS = new Set(['layout', 'theme', 'sections']);

function groupFields(entries: [string, LaunchFormInput][]) {
  const core: typeof entries = [];
  const style: typeof entries = [];
  const extra: typeof entries = [];
  for (const entry of entries) {
    const [key] = entry;
    if (CORE_KEYS.has(key)) core.push(entry);
    else if (STYLE_KEYS.has(key)) style.push(entry);
    else extra.push(entry);
  }
  return { core, style, extra };
}

function FieldControl({
  fieldKey,
  input,
  value,
  onChange,
}: {
  fieldKey: string;
  input: LaunchFormInput;
  value: string;
  onChange: (v: string) => void;
}) {
  if (input.options) {
    return (
      <Select value={toSelectValue(value)} onValueChange={(v) => onChange(fromSelectValue(v))}>
        <SelectTrigger id={`launch-${fieldKey}`} className={cn(formControlClass, 'cursor-pointer')}>
          <SelectValue placeholder="请选择" />
        </SelectTrigger>
        <SelectContent className="max-h-[min(280px,50vh)]">
          {input.options.map((opt) => (
            <SelectItem
              key={opt.value || '__auto__'}
              value={toSelectValue(opt.value)}
              className="cursor-pointer rounded-md py-2 pl-8 pr-3"
            >
              {opt.label ?? (opt.value || '自动选择')}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    );
  }

  return (
    <>
      <textarea
        id={`launch-${fieldKey}`}
        className={formTextareaClass}
        placeholder={input.description ?? `请输入${fieldLabel(fieldKey)}`}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      {input.examples && input.examples.length > 0 && (
        <div className="pt-1">
          <p className="mb-2 flex items-center gap-1 text-xs text-muted-foreground">
            <Sparkles className="h-3 w-3 shrink-0 opacity-70" />
            快速示例
          </p>
          <div className="flex flex-wrap gap-1.5">
            {input.examples.map((ex) => {
              const active = value === ex.value;
              return (
                <button
                  key={ex.value}
                  type="button"
                  title={ex.value}
                  onClick={() => onChange(ex.value)}
                  className={cn(
                    'rounded-md border px-2.5 py-1 text-xs transition-colors',
                    active
                      ? 'border-primary bg-primary/10 text-primary font-medium'
                      : 'border-border bg-muted/40 text-muted-foreground hover:border-primary/40 hover:text-foreground',
                  )}
                >
                  {ex.label ?? ex.value}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </>
  );
}

function FieldGrid({
  entries,
  values,
  onChange,
}: {
  entries: [string, LaunchFormInput][];
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
}) {
  if (entries.length === 0) return null;

  const isPairRow = entries.length === 2 && entries.every(([, i]) => i.options);

  if (isPairRow) {
    return (
      <div className="grid gap-4 sm:grid-cols-2">
        {entries.map(([key, input]) => (
          <FormField
            key={key}
            label={fieldLabel(key)}
            htmlFor={`launch-${key}`}
            required={input.required}
            hint={input.description}
          >
            <FieldControl fieldKey={key} input={input} value={values[key] ?? ''} onChange={(v) => onChange(key, v)} />
          </FormField>
        ))}
      </div>
    );
  }

  return (
    <>
      {entries.map(([key, input]) => (
        <FormField
          key={key}
          label={fieldLabel(key)}
          htmlFor={`launch-${key}`}
          required={input.required}
          hint={input.options ? input.description : undefined}
        >
          <FieldControl fieldKey={key} input={input} value={values[key] ?? ''} onChange={(v) => onChange(key, v)} />
        </FormField>
      ))}
    </>
  );
}

export function LaunchFormFields({ inputs, values, onChange }: LaunchFormFieldsProps) {
  const entries = sortFieldEntries(Object.entries(inputs));

  if (entries.length === 0) {
    return (
      <p className="py-8 text-center text-sm text-muted-foreground">
        无需填写参数，点击下方按钮即可运行。
      </p>
    );
  }

  const { core, style, extra } = groupFields(entries);
  const hasRequired = entries.some(([, i]) => i.required);

  return (
    <div className="space-y-8">
      {core.length > 0 && (
        <FormSection title="基本信息" description="描述你要生成的站点或页面">
          <FieldGrid entries={core} values={values} onChange={onChange} />
        </FormSection>
      )}
      {style.length > 0 && (
        <FormSection title="风格与布局" description="留空则由 AI 根据描述自动选择">
          <FieldGrid entries={style} values={values} onChange={onChange} />
        </FormSection>
      )}
      {extra.length > 0 && (
        <FormSection title="可选项">
          <FieldGrid entries={extra} values={values} onChange={onChange} />
        </FormSection>
      )}
      {hasRequired && (
        <p className="text-xs text-muted-foreground">标有 * 的字段为必填</p>
      )}
    </div>
  );
}
