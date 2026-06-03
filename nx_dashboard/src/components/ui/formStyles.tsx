import type { ReactNode } from 'react';

/** 表单分区标题 */
export function FormSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="space-y-4">
      <div className="border-b border-border/50 pb-2">
        <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        {description && (
          <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      <div className="space-y-4">{children}</div>
    </section>
  );
}

/** 单行表单字段：标签 + 说明 + 控件，无套娃卡片 */
export function FormField({
  label,
  htmlFor,
  required,
  hint,
  children,
}: {
  label: string;
  htmlFor?: string;
  required?: boolean;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <label htmlFor={htmlFor} className="text-sm font-medium text-foreground">
          {label}
        </label>
        {required && <span className="text-destructive text-xs">*</span>}
      </div>
      {hint && <p className="text-xs leading-relaxed text-muted-foreground">{hint}</p>}
      {children}
    </div>
  );
}

export const formControlClass =
  'w-full h-11 rounded-lg border border-input bg-background px-3 text-sm shadow-sm transition-colors placeholder:text-muted-foreground/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:border-primary/50';

export const formTextareaClass =
  'w-full min-h-[5.5rem] resize-y rounded-lg border border-input bg-background px-3 py-2.5 text-sm leading-relaxed shadow-sm transition-colors placeholder:text-muted-foreground/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:border-primary/50';

/** @deprecated 使用 FormField + formControlClass */
export const formFieldClass = formControlClass;

/** @deprecated 不再使用套娃卡片 */
export const formCardClass = '';

export const formLabelClass = 'text-sm font-medium text-foreground';

export const formHintClass = 'text-xs leading-relaxed text-muted-foreground';
