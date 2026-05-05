import React from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { API_BASE_URL } from '../../api/constants';
import { Button } from '@/components/ui/button';

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
        setInterrupted((prev) => prev.filter((e) => e.execution_id !== executionId));
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
        setInterrupted((prev) => prev.filter((e) => e.execution_id !== executionId));
      }
    } catch {
      // silent
    }
  };

  if (loading || interrupted.length === 0) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: -8 }}
      animate={{ opacity: 1, y: 0 }}
      className="border border-warning/40 rounded-lg bg-warning/5 p-3 space-y-2"
    >
      <h4 className="text-sm font-semibold text-yellow-600 dark:text-yellow-400">
        检测到 {interrupted.length} 个中断任务
      </h4>
      <p className="text-xs text-muted-foreground">
        以下任务在上次运行中被意外中断，可以选择继续或放弃。
      </p>

      <div className="space-y-2 max-h-40 overflow-y-auto">
        <AnimatePresence>
          {interrupted.map((exec) => (
            <motion.div
              key={exec.execution_id}
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 8 }}
              className="p-2 bg-card rounded border border-border"
            >
              <p className="text-xs font-medium truncate">{exec.task_prompt}</p>
              <p className="text-xs text-muted-foreground mt-0.5">
                中断于: {new Date(exec.last_heartbeat).toLocaleString()}
              </p>
              <div className="flex gap-2 mt-1.5">
                <Button
                  size="sm"
                  className="h-6 text-xs px-2"
                  onClick={() => handleResume(exec.execution_id)}
                  disabled={resuming === exec.execution_id}
                >
                  {resuming === exec.execution_id ? '恢复中...' : '继续执行'}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  className="h-6 text-xs px-2"
                  onClick={() => handleAbandon(exec.execution_id)}
                >
                  放弃
                </Button>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}
