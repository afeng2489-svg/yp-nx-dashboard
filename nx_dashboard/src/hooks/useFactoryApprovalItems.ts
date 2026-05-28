import { useMemo } from 'react';
import { useExecutionStore } from '@/stores/executionStore';

export interface FactoryApprovalItem {
  executionId: string;
  stageName: string;
  question: string;
  pauseKind: string;
}

export function useFactoryApprovalItems(): FactoryApprovalItem[] {
  const executions = useExecutionStore((s) => s.executions);
  const pendingPause = useExecutionStore((s) => s.pendingPause);

  return useMemo(() => {
    const paused = executions.filter((e) => e.status === 'paused');
    const items: FactoryApprovalItem[] = [];

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

    if (pendingPause && !items.some((i) => i.executionId === pendingPause.execution_id)) {
      items.unshift({
        executionId: pendingPause.execution_id,
        stageName: pendingPause.stage_name,
        question: pendingPause.question,
        pauseKind: pendingPause.pause_kind ?? 'user_input',
      });
    }

    return items;
  }, [executions, pendingPause]);
}
