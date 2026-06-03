import { useState, useEffect } from 'react';
import { Play, Loader2, AlertTriangle } from 'lucide-react';
import { useExecutionStore, WorkspaceConflictError } from '@/stores/executionStore';
import { useNavigate } from 'react-router-dom';
import { API_BASE_URL } from '@/api/constants';
import { showError, showSuccess } from '@/lib/toast';
import { LaunchModalShell } from './LaunchModalShell';
import { LaunchModalFooter } from './LaunchModalFooter';
import { LaunchFormFields, type LaunchFormInput } from './LaunchFormFields';
import { workflowTitle } from './launchFormUtils';

interface LaunchWorkflow {
  id: string;
  name: string;
  description?: string;
}

interface WorkflowLaunchModalProps {
  workflow: LaunchWorkflow;
  onClose: () => void;
}

export function WorkflowLaunchModal({ workflow, onClose }: WorkflowLaunchModalProps) {
  const navigate = useNavigate();
  const { startExecution, connectWebSocket } = useExecutionStore();

  const [inputs, setInputs] = useState<Record<string, LaunchFormInput>>({});
  const [values, setValues] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);
  const [conflict, setConflict] = useState<{ message: string; files: string[] } | null>(null);

  useEffect(() => {
    const fetchInputs = async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${workflow.id}`);
        if (!res.ok) return;
        const full = await res.json();
        const wf = full.data ?? full;
        const triggers = wf.definition?.triggers ?? wf.triggers ?? [];
        const wfInputs: Record<string, LaunchFormInput> = triggers[0]?.inputs ?? {};
        setInputs(wfInputs);
        const initial: Record<string, string> = {};
        Object.entries(wfInputs).forEach(([k, input]) => {
          initial[k] = input.options?.[0]?.value ?? '';
        });
        setValues(initial);
      } finally {
        setLoading(false);
      }
    };
    fetchInputs();
  }, [workflow.id]);

  const handleExecute = async (confirmOverwrite = false) => {
    for (const [key, input] of Object.entries(inputs)) {
      if (input.required && !values[key]?.trim()) {
        showError(`请填写「${key}」`);
        return;
      }
    }
    setExecuting(true);
    try {
      const execution = await startExecution(workflow.id, values as Record<string, unknown>, {
        confirmOverwrite,
      });
      connectWebSocket(execution.id);
      showSuccess(`工作流「${workflowTitle(workflow.name)}」已启动`);
      onClose();
      navigate('/factory?tab=runs');
    } catch (e) {
      if (e instanceof WorkspaceConflictError) {
        setConflict({ message: e.message, files: e.files });
      } else {
        showError(`执行失败: ${e}`);
      }
    } finally {
      setExecuting(false);
    }
  };

  return (
    <LaunchModalShell
      onClose={onClose}
      title={workflowTitle(workflow.name)}
      subtitle={workflow.description || '填写参数后启动产线'}
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={() => handleExecute(false)}
          cancelLabel="取消"
          submitLabel="开始生成"
          submitting={executing}
          disabled={loading}
          submitIcon={!executing ? <Play className="h-4 w-4" /> : undefined}
        />
      }
      overlay={
        conflict ? (
          <div className="absolute inset-0 z-10 flex items-center justify-center bg-background/90 p-5 backdrop-blur-sm">
            <div className="w-full max-w-md space-y-4 rounded-xl border border-border bg-card p-5 shadow-lg">
              <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
                <AlertTriangle className="h-5 w-5 shrink-0" />
                <h3 className="font-semibold">目标文件夹非空</h3>
              </div>
              <p className="text-sm text-muted-foreground">{conflict.message}</p>
              {conflict.files.length > 0 && (
                <div className="max-h-36 overflow-auto rounded-lg border border-border bg-muted/30 p-3">
                  <div className="flex flex-wrap gap-1.5">
                    {conflict.files.map((f) => (
                      <code key={f} className="rounded bg-muted px-1.5 py-0.5 text-[11px]">
                        {f}
                      </code>
                    ))}
                  </div>
                </div>
              )}
              <LaunchModalFooter
                onCancel={() => setConflict(null)}
                onSubmit={() => {
                  setConflict(null);
                  void handleExecute(true);
                }}
                cancelLabel="返回修改"
                submitLabel="仍要继续"
                submitting={executing}
              />
            </div>
          </div>
        ) : undefined
      }
    >
      {loading ? (
        <div className="flex items-center justify-center py-16">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <LaunchFormFields
          inputs={inputs}
          values={values}
          onChange={(key, value) => setValues((prev) => ({ ...prev, [key]: value }))}
        />
      )}
    </LaunchModalShell>
  );
}
