import { useExecutionStore } from '@/stores/executionStore';
import { AlertCircle, CheckCircle2 } from 'lucide-react';

/** 待审批 / user_input 暂停（AF-01 Approvals Tab MVP） */
export function FactoryApprovalsTab() {
  const pendingPause = useExecutionStore((s) => s.pendingPause);
  const paused = useExecutionStore((s) =>
    s.executions.filter((e) => e.status === 'paused'),
  );

  if (!pendingPause && paused.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
        <CheckCircle2 className="w-12 h-12 mb-3 opacity-40" />
        <p className="text-sm">暂无待审批项</p>
        <p className="text-xs mt-1">工作流等待输入时会出现在此处或右下角浮层</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {pendingPause && (
        <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
          <div className="flex items-start gap-3">
            <AlertCircle className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
            <div>
              <p className="font-medium text-sm">等待你的回复</p>
              <p className="text-xs text-muted-foreground mt-1">阶段: {pendingPause.stage_name}</p>
              <p className="text-sm mt-2">{pendingPause.question}</p>
              <p className="text-xs text-muted-foreground mt-2">请使用右下角审批卡片操作</p>
            </div>
          </div>
        </div>
      )}
      {paused.map((e) => (
        <div key={e.id} className="rounded-xl border border-border p-4">
          <p className="text-sm font-medium">Run {e.id.slice(0, 8)}…</p>
          <p className="text-xs text-muted-foreground">状态: 已暂停</p>
        </div>
      ))}
    </div>
  );
}
