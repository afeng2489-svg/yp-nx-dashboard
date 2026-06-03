import { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';

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

  return (
    <LaunchModalShell
      onClose={onCancel}
      title={title}
      icon={<AlertTriangle />}
      accent={variant === 'info' ? 'indigo' : 'amber'}
      size="md"
      footer={
        <LaunchModalFooter
          onCancel={onCancel}
          onSubmit={() => {
            onConfirm();
            onCancel();
          }}
          cancelLabel={cancelText}
          submitLabel={confirmText}
        />
      }
    >
      <p className="text-sm leading-relaxed text-muted-foreground">{message}</p>
    </LaunchModalShell>
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
