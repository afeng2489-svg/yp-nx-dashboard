import { useMemo, useSyncExternalStore } from 'react';
import { useExecutionStore, type Execution } from '@/stores/executionStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import {
  getRunOutcomeDismissEpoch,
  isRunOutcomeDismissed,
  subscribeRunOutcomeDismiss,
} from '@/data/runNextSteps';

export function terminalFactoryRuns(executions: Execution[]): Execution[] {
  return executions
    .filter(
      (e) =>
        e.trigger_source === 'factory' &&
        (e.status === 'completed' || e.status === 'failed'),
    )
    .sort((a, b) => {
      const ta = a.finished_at
        ? Date.parse(a.finished_at)
        : a.started_at
          ? Date.parse(a.started_at)
          : 0;
      const tb = b.finished_at
        ? Date.parse(b.finished_at)
        : b.started_at
          ? Date.parse(b.started_at)
          : 0;
      return tb - ta;
    });
}

/** 最近一条可展示 next-step / 失败恢复的工厂 Run */
export function useRecentFactoryOutcome() {
  const executions = useExecutionStore((s) => s.executions);
  const workflows = useWorkflowStore((s) => s.workflows);
  const dismissEpoch = useSyncExternalStore(
    subscribeRunOutcomeDismiss,
    getRunOutcomeDismissEpoch,
    getRunOutcomeDismissEpoch,
  );

  return useMemo(() => {
    const latest = terminalFactoryRuns(executions)[0];
    if (!latest || isRunOutcomeDismissed(latest.id)) return null;

    const wf = workflows.find((w) => w.id === latest.workflow_id);
    const workflowName = wf?.name ?? 'solo-dev';

    return { execution: latest, workflowName };
  }, [executions, workflows, dismissEpoch]);
}
