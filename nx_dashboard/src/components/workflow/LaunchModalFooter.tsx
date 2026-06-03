import type { ReactNode } from 'react';
import { Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface LaunchModalFooterProps {
  onCancel: () => void;
  onSubmit: () => void;
  cancelLabel?: string;
  submitLabel?: string;
  submitting?: boolean;
  disabled?: boolean;
  submitIcon?: ReactNode;
  /** 左下角提示，如「带 * 为必填项」 */
  hint?: string;
}

/** 弹窗底部统一操作栏：左提示 + 右主次按钮，等高、对齐、无渐变花活 */
export function LaunchModalFooter({
  onCancel,
  onSubmit,
  cancelLabel = '取消',
  submitLabel = '确认',
  submitting = false,
  disabled = false,
  submitIcon,
  hint,
}: LaunchModalFooterProps) {
  return (
    <div className="flex flex-col-reverse gap-3 sm:flex-row sm:items-center sm:justify-between">
      {hint ? (
        <p className="text-center text-xs text-muted-foreground sm:text-left">{hint}</p>
      ) : (
        <span className="hidden sm:block" />
      )}
      <div className="flex w-full flex-col-reverse gap-2.5 sm:w-auto sm:flex-row sm:items-center">
        <button
          type="button"
          onClick={onCancel}
          disabled={submitting}
          className="btn-ghost h-11 w-full px-5 text-sm sm:w-auto"
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          onClick={onSubmit}
          disabled={disabled || submitting}
          className={cn(
            'btn-primary h-11 w-full min-w-[8.5rem] px-6 text-sm sm:w-auto',
            (disabled || submitting) && 'opacity-50',
          )}
        >
          {submitting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            submitIcon
          )}
          {submitting ? '处理中…' : submitLabel}
        </button>
      </div>
    </div>
  );
}
