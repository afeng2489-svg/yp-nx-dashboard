import { useEffect, useCallback, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils';

type ModalSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  children: ReactNode;
  size?: ModalSize;
  showCloseButton?: boolean;
  closeOnBackdrop?: boolean;
  closeOnEsc?: boolean;
  className?: string;
  footer?: ReactNode;
}

const SIZE_CLASSES: Record<ModalSize, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-xl',
  full: 'max-w-4xl',
};

export function Modal({
  isOpen,
  onClose,
  title,
  description,
  children,
  size = 'md',
  showCloseButton = true,
  closeOnBackdrop = true,
  closeOnEsc = true,
  className,
  footer,
}: ModalProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (closeOnEsc && e.key === 'Escape') {
        onClose();
      }
    },
    [closeOnEsc, onClose],
  );

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [isOpen, handleKeyDown]);

  if (!isOpen) return null;

  return createPortal(
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-full items-center justify-center p-4 sm:p-6">
        <div
          className="absolute inset-0 bg-black/50 backdrop-blur-md"
          onClick={closeOnBackdrop ? onClose : undefined}
          aria-hidden
        />
        <div
          role="dialog"
          aria-modal
          className={cn(
            'relative flex w-full flex-col overflow-hidden rounded-2xl border border-border/60 bg-card/95 shadow-2xl backdrop-blur-xl',
            'animate-in fade-in zoom-in-95 duration-200',
            'max-h-[min(92vh,860px)]',
            SIZE_CLASSES[size],
            className,
          )}
        >
          {(title || showCloseButton) && (
            <div className="flex shrink-0 items-start justify-between border-b border-border/50 px-6 py-4">
              <div className="min-w-0 pr-4">
                {title && <h3 className="text-lg font-semibold tracking-tight">{title}</h3>}
                {description && (
                  <p className="mt-1 text-sm text-muted-foreground">{description}</p>
                )}
              </div>
              {showCloseButton && (
                <button
                  type="button"
                  onClick={onClose}
                  className="shrink-0 rounded-lg p-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                  aria-label="关闭"
                >
                  <X className="h-5 w-5" />
                </button>
              )}
            </div>
          )}
          <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-6 py-4">
            {children}
          </div>
          {footer && (
            <div className="shrink-0 border-t border-border/50 bg-muted/20 px-6 py-4">
              {footer}
            </div>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}
