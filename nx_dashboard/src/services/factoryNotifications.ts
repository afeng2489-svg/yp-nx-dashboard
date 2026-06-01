import { useExecutionStore } from '@/stores/executionStore';
import { useSettingsStore } from '@/stores/settingsStore';

let lastNotifiedApprovalId: string | null = null;

/** AF-UX-08：待批准桌面通知（Tauri / 浏览器 fallback） */
export async function notifyPendingApproval(executionId: string, question: string): Promise<void> {
  const { notifications, factory } = useSettingsStore.getState();
  if (!factory.approvalDesktopNotify || !notifications.executionComplete) return;
  if (lastNotifiedApprovalId === executionId) return;
  lastNotifiedApprovalId = executionId;

  const title = 'TeamFlow · 待批准';
  const body = question.slice(0, 120) || '工厂 Run 等待你的审批';

  try {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(title, { body });
      return;
    }
    if ('Notification' in window && Notification.permission === 'default') {
      const perm = await Notification.requestPermission();
      if (perm === 'granted') {
        new Notification(title, { body });
      }
    }
  } catch {
    /* ignore */
  }
}

/** 监听 execution store 中的 paused approval */
export function watchFactoryApprovals(): () => void {
  return useExecutionStore.subscribe((state) => {
    const paused = state.executions.find(
      (e) =>
        e.status === 'paused' &&
        (e.pending_pause?.pause_kind === 'approval' ||
          state.pendingPause?.execution_id === e.id),
    );
    if (paused?.pending_pause?.pause_kind === 'approval') {
      void notifyPendingApproval(paused.id, paused.pending_pause.question);
    }
  });
}
