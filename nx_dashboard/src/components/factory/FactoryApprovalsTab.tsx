import { useMemo } from 'react';
import { CheckCircle2 } from 'lucide-react';
import { useExecutionStore } from '@/stores/executionStore';
import { ApprovalPanel } from '@/components/factory/ApprovalPanel';

/** 待审批 — 对接 resolve API */
export function FactoryApprovalsTab() {
  const executions = useExecutionStore((s) => s.executions);
  const pendingPause = useExecutionStore((s) => s.pendingPause);

  const approvalItems = useMemo(() => {
    const paused = executions.filter((e) => e.status === 'paused');
    const items: {
      executionId: string;
      stageName: string;
      question: string;
      pauseKind: string;
    }[] = [];

    for (const exec of paused) {
      const pause = exec.id === pendingPause?.execution_id ? pendingPause : exec.pending_pause;
      if (!pause) continue;
      items.push({
        executionId: exec.id,
        stageName: pause.stage_name,
        question: pause.question,
        pauseKind: pause.pause_kind ?? 'user_input',
      });
    }

    if (
      pendingPause &&
      !items.some((i) => i.executionId === pendingPause.execution_id)
    ) {
      items.unshift({
        executionId: pendingPause.execution_id,
        stageName: pendingPause.stage_name,
        question: pendingPause.question,
        pauseKind: pendingPause.pause_kind ?? 'user_input',
      });
    }

    return items;
  }, [executions, pendingPause]);

  if (approvalItems.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
        <CheckCircle2 className="w-12 h-12 mb-3 opacity-40" />
        <p className="text-sm">暂无待审批项</p>
        <p className="text-xs mt-1">solo-dev 等工作流在「交付审批」阶段会暂停在此</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {approvalItems.map((item) =>
        item.pauseKind === 'approval' ? (
          <ApprovalPanel
            key={item.executionId}
            executionId={item.executionId}
            stageName={item.stageName}
            question={item.question}
          />
        ) : (
          <div
            key={item.executionId}
            className="rounded-xl border border-border p-4 text-sm text-muted-foreground"
          >
            <p className="font-medium text-foreground">Run {item.executionId.slice(0, 8)}…</p>
            <p className="text-xs mt-1">{item.stageName}</p>
            <p className="mt-2">{item.question}</p>
            <p className="text-xs mt-2">此为选项型输入，请使用右下角浮层操作</p>
          </div>
        ),
      )}
    </div>
  );
}
