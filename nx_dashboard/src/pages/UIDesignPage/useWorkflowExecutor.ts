import { useState, useEffect } from 'react';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import { showError, showSuccess } from '@/lib/toast';

// ── 工作流执行 Hook ──────────────────────────────────
export function useWorkflowExecutor() {
  const { workflows, fetchWorkflows } = useWorkflowStore();
  const { startExecution } = useExecutionStore();
  const [running, setRunning] = useState<string | null>(null);

  useEffect(() => {
    fetchWorkflows();
  }, [fetchWorkflows]);

  // Returns executionId on success, null on failure
  const execute = async (
    wfName: string,
    variables: Record<string, string>,
  ): Promise<string | null> => {
    const wf = workflows.find((w) => w.name === wfName);
    if (!wf) {
      showError(`工作流 "${wfName}" 未找到，请重启后端以导入`);
      return null;
    }
    setRunning(wfName);
    try {
      const execution = await startExecution(wf.id, variables as Record<string, unknown>);
      showSuccess(`"${wfName}" 已启动`);
      return execution.id;
    } catch (e) {
      showError(`启动失败: ${e}`);
      return null;
    } finally {
      setRunning(null);
    }
  };

  return { running, execute };
}
