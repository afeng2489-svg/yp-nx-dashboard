import { useState } from 'react';
import { AlertTriangle, X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ConfirmModalProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  variant?: 'danger' | 'warning' | 'info';
}

export function ConfirmModal({
  isOpen,
  title,
  message,
  confirmText = '确认',
  cancelText = '取消',
  onConfirm,
  onCancel,
  variant = 'danger',
}: ConfirmModalProps) {
  if (!isOpen) return null;

  const variantStyles = {
    danger: {
      icon: 'text-red-500',
      iconBg: 'bg-red-500/10',
      confirmBtn: 'bg-red-500 hover:bg-red-600 text-white',
    },
    warning: {
      icon: 'text-amber-500',
      iconBg: 'bg-amber-500/10',
      confirmBtn: 'bg-amber-500 hover:bg-amber-600 text-white',
    },
    info: {
      icon: 'text-blue-500',
      iconBg: 'bg-blue-500/10',
      confirmBtn: 'bg-blue-500 hover:bg-blue-600 text-white',
    },
  };

  const styles = variantStyles[variant];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onCancel} />
      <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden animate-in fade-in zoom-in duration-200">
        <div className="p-6">
          <div className="flex items-start gap-4">
            <div className={cn('p-3 rounded-xl', styles.iconBg)}>
              <AlertTriangle className={cn('w-6 h-6', styles.icon)} />
            </div>
            <div className="flex-1">
              <h3 className="text-lg font-semibold mb-2">{title}</h3>
              <p className="text-sm text-muted-foreground">{message}</p>
            </div>
            <button onClick={onCancel} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50 bg-gradient-to-r from-muted/50 to-muted/30">
          <button onClick={onCancel} className="btn-secondary">
            {cancelText}
          </button>
          <button
            onClick={() => { onConfirm(); onCancel(); }}
            className={cn('px-4 py-2 rounded-lg font-medium transition-colors', styles.confirmBtn)}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useConfirmModal() {
  const [confirmState, setConfirmState] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
    variant?: 'danger' | 'warning' | 'info';
  }>({
    isOpen: false,
    title: '',
    message: '',
    onConfirm: () => {},
  });

  const showConfirm = (
    title: string,
    message: string,
    onConfirm: () => void,
    variant: 'danger' | 'warning' | 'info' = 'danger',
  ) => {
    setConfirmState({ isOpen: true, title, message, onConfirm, variant });
  };

  const hideConfirm = () => {
    setConfirmState((prev) => ({ ...prev, isOpen: false }));
  };

  return { confirmState, showConfirm, hideConfirm };
}
