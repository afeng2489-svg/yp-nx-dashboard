import { toast } from 'sonner';

// Toast configuration
const toastConfig = {
  duration: 4000,
  dismissible: true,
};

// Success toast with gradient styling
export function showSuccess(message: string, description?: string) {
  return toast.success(message, {
    ...toastConfig,
    description,
    classNames: {
      toast: 'bg-gradient-to-r from-emerald-500/10 to-green-500/10 border border-emerald-500/20',
      title: 'text-emerald-600 font-medium',
      description: 'text-emerald-600/70',
    },
  });
}

// Error toast with gradient styling
export function showError(message: string, description?: string) {
  return toast.error(message, {
    ...toastConfig,
    description,
    classNames: {
      toast: 'bg-gradient-to-r from-red-500/10 to-rose-500/10 border border-red-500/20',
      title: 'text-red-600 font-medium',
      description: 'text-red-600/70',
    },
  });
}

// Info toast with gradient styling
export function showInfo(message: string, description?: string) {
  return toast(message, {
    ...toastConfig,
    description,
    classNames: {
      toast: 'bg-gradient-to-r from-indigo-500/10 to-purple-500/10 border border-indigo-500/20',
      title: 'text-indigo-600 font-medium',
      description: 'text-indigo-600/70',
    },
  });
}

// Warning toast with gradient styling
export function showWarning(message: string, description?: string) {
  return toast.warning(message, {
    ...toastConfig,
    description,
    classNames: {
      toast: 'bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/20',
      title: 'text-amber-600 font-medium',
      description: 'text-amber-600/70',
    },
  });
}

// Loading toast (promise)
export function showLoading(
  message: string,
  promise: Promise<unknown>
): Promise<unknown> {
  toast.promise(promise, {
    loading: message,
    success: () => `${message} 成功`,
    error: (err) => `${message} 失败: ${err}`,
    classNames: {
      toast: 'bg-gradient-to-r from-indigo-500/10 to-purple-500/10 border border-indigo-500/20',
    },
  });
  return promise;
}

// Custom toast with action
export function showAction(
  message: string,
  action: {
    label: string;
    onClick: () => void;
  }
) {
  return toast(message, {
    ...toastConfig,
    action: {
      label: action.label,
      onClick: action.onClick,
    },
    classNames: {
      toast: 'bg-card border border-border/50 shadow-lg',
      title: 'font-medium',
    },
  });
}
