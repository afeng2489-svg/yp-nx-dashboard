import { useExecutionStore } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { WorkflowPauseCardInline } from './WorkflowPauseCard';
import { TeamTaskCardInline } from './TeamTaskCard';

/**
 * GlobalOpsOverlay — 挂载在 Dashboard 布局根节点
 * 所有全局悬浮卡统一从右下角向上堆叠。
 */
export function GlobalOpsOverlay() {
  const pendingPause = useExecutionStore((s) => s.pendingPause);
  const activeTeamTask = useTeamStore((s) => s.activeTeamTask);

  if (!pendingPause && !activeTeamTask) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col-reverse gap-3 items-end">
      {pendingPause && <WorkflowPauseCardInline />}
      {activeTeamTask && <TeamTaskCardInline />}
    </div>
  );
}
