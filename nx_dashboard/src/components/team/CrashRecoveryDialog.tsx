import React from 'react';
import { API_BASE_URL } from '../../api/constants';

interface InterruptedExecution {
  id: string;
  execution_id: string;
  project_id: string;
  role_id: string;
  task_prompt: string;
  accumulated_output: string;
  phase: string;
  started_at: string;
  last_heartbeat: string;
}

interface Props {
  projectId?: string;
  onResumed?: () => void;
}

export default function CrashRecoveryDialog({ projectId, onResumed }: Props) {
  const [interrupted, setInterrupted] = React.useState<InterruptedExecution[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [resuming, setResuming] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!projectId) {
      setLoading(false);
      return;
    }
    (async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/executions/interrupted`);
        if (res.ok) {
          const data = await res.json();
          setInterrupted(data.filter((e: InterruptedExecution) => e.project_id === projectId));
        }
      } catch {
        // silent
      }
      setLoading(false);
    })();
  }, [projectId]);

  const handleResume = async (executionId: string) => {
    setResuming(executionId);
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/executions/${executionId}/resume`, {
        method: 'POST',
      });
      if (res.ok) {
        setInterrupted(prev => prev.filter(e => e.execution_id !== executionId));
        onResumed?.();
      }
    } catch {
      // silent
    }
    setResuming(null);
  };

  const handleAbandon = async (executionId: string) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/executions/${executionId}/checkpoint`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setInterrupted(prev => prev.filter(e => e.execution_id !== executionId));
      }
    } catch {
      // silent
    }
  };

  if (loading || interrupted.length === 0) return null;

  return (
    <div className="border border-orange-200 dark:border-orange-800 rounded-lg bg-orange-50 dark:bg-orange-950/30 p-3 space-y-2">
      <h4 className="text-sm font-semibold text-orange-600 dark:text-orange-400">
        检测到 {interrupted.length} 个中断任务
      </h4>
      <p className="text-xs text-orange-600/70 dark:text-orange-400/70">
        以下任务在上次运行中被意外中断，可以选择继续或放弃。
      </p>

      <div className="space-y-2 max-h-40 overflow-y-auto">
        {interrupted.map(exec => (
          <div key={exec.execution_id} className="p-2 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700">
            <p className="text-xs font-medium truncate">{exec.task_prompt}</p>
            <p className="text-xs text-gray-400 mt-0.5">
              中断于: {new Date(exec.last_heartbeat).toLocaleString()}
            </p>
            <div className="flex gap-2 mt-1.5">
              <button
                className="px-2 py-0.5 text-xs rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                onClick={() => handleResume(exec.execution_id)}
                disabled={resuming === exec.execution_id}
              >
                {resuming === exec.execution_id ? '恢复中...' : '继续执行'}
              </button>
              <button
                className="px-2 py-0.5 text-xs rounded bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-300"
                onClick={() => handleAbandon(exec.execution_id)}
              >
                放弃
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
