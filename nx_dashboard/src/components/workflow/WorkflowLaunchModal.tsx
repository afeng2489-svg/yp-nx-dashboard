import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { X, Play, Loader2 } from 'lucide-react';
import { useExecutionStore } from '@/stores/executionStore';
import { useNavigate } from 'react-router-dom';
import { API_BASE_URL } from '@/api/constants';
import { showError, showSuccess } from '@/lib/toast';

interface WorkflowInput {
  type: string;
  required?: boolean;
  description?: string;
}

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

  const [inputs, setInputs] = useState<Record<string, WorkflowInput>>({});
  const [values, setValues] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  useEffect(() => {
    const fetchInputs = async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${workflow.id}`);
        if (!res.ok) return;
        const full = await res.json();
        const triggers = full.definition?.triggers ?? full.triggers ?? [];
        const wfInputs: Record<string, WorkflowInput> = triggers[0]?.inputs ?? {};
        setInputs(wfInputs);
        const initial: Record<string, string> = {};
        Object.keys(wfInputs).forEach((k) => {
          initial[k] = '';
        });
        setValues(initial);
      } finally {
        setLoading(false);
      }
    };
    fetchInputs();
  }, [workflow.id]);

  const handleExecute = async () => {
    for (const [key, input] of Object.entries(inputs)) {
      if (input.required && !values[key]?.trim()) {
        showError(`请填写 "${key}"`);
        return;
      }
    }
    setExecuting(true);
    try {
      const execution = await startExecution(workflow.id, values as Record<string, unknown>);
      connectWebSocket(execution.id);
      showSuccess(`工作流 "${workflow.name}" 已启动`);
      onClose();
      navigate('/executions');
    } catch (e) {
      showError(`执行失败: ${e}`);
    } finally {
      setExecuting(false);
    }
  };

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-emerald-500/5 to-green-500/5">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-emerald-500 to-green-500 shadow-lg shadow-emerald-500/25">
              <Play className="w-4 h-4 text-white" />
            </div>
            <div>
              <h2 className="font-semibold">{workflow.name}</h2>
              <p className="text-xs text-muted-foreground">
                {workflow.description || '启动工作流'}
              </p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-6">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
            </div>
          ) : Object.keys(inputs).length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">
              此工作流无需输入参数，点击执行即可直接运行。
            </p>
          ) : (
            <div className="space-y-4">
              {Object.entries(inputs).map(([key, input]) => (
                <div key={key}>
                  <label className="block text-sm font-medium mb-1">
                    {key}
                    {input.required && <span className="text-red-500 ml-1">*</span>}
                  </label>
                  {input.description && (
                    <p className="text-xs text-muted-foreground mb-1.5">{input.description}</p>
                  )}
                  <textarea
                    className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none min-h-[80px]"
                    placeholder={input.description ?? `输入 ${key}…`}
                    value={values[key] ?? ''}
                    onChange={(e) => setValues((prev) => ({ ...prev, [key]: e.target.value }))}
                  />
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border/50">
          <button onClick={onClose} className="btn-secondary">
            取消
          </button>
          <button
            onClick={handleExecute}
            disabled={executing || loading}
            className="btn-primary flex items-center gap-2"
          >
            {executing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Play className="w-4 h-4" />
            )}
            {executing ? '启动中…' : '执行'}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
