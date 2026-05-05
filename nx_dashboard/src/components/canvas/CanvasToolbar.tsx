import { useState } from 'react';
import { useCanvasStore } from '@/stores/canvasStore';
import { api } from '@/api/client';
import { toast } from 'sonner';
import { TemplatesPanel } from './TemplatesPanel';
import { WorkflowLaunchModal } from '@/components/workflow/WorkflowLaunchModal';

export function CanvasToolbar() {
  const {
    workflowName,
    workflowId,
    setWorkflowName,
    setWorkflowId,
    toYaml,
    loadFromYaml,
    resetExecStatus,
  } = useCanvasStore();
  const [saving, setSaving] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const [showLaunch, setShowLaunch] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      const definition = toYaml();
      if (workflowId) {
        await api.updateWorkflow(workflowId, { name: workflowName, definition });
      } else {
        const wf = await api.createWorkflow({ name: workflowName, definition });
        setWorkflowId(wf.id);
      }
      toast.success('已保存');
    } catch {
      toast.error('保存失败');
    } finally {
      setSaving(false);
    }
  };

  const run = async () => {
    if (!workflowId) {
      // 先保存再弹出
      setSaving(true);
      try {
        const definition = toYaml();
        const wf = await api.createWorkflow({ name: workflowName, definition });
        setWorkflowId(wf.id);
      } catch {
        toast.error('保存失败，无法运行');
        setSaving(false);
        return;
      }
      setSaving(false);
    }
    resetExecStatus();
    setShowLaunch(true);
  };

  const importFile = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.yaml,.yml';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      loadFromYaml(text);
    };
    input.click();
  };

  return (
    <>
      {showTemplates && <TemplatesPanel onClose={() => setShowTemplates(false)} />}
      {showLaunch && workflowId && (
        <WorkflowLaunchModal
          workflow={{ id: workflowId, name: workflowName }}
          onClose={() => setShowLaunch(false)}
        />
      )}
      <div className="flex items-center gap-2 border-b border-border bg-card px-4 py-2">
        <input
          className="rounded bg-background px-2 py-1 text-sm border border-border focus:outline-none focus:border-primary w-48"
          value={workflowName}
          onChange={(e) => setWorkflowName(e.target.value)}
        />
        <div className="flex-1" />
        <button onClick={() => setShowTemplates(true)} className={BTN}>
          模板库
        </button>
        <button onClick={importFile} className={BTN}>
          导入 YAML
        </button>
        <button onClick={save} disabled={saving} className={`${BTN} ${saving ? 'opacity-50' : ''}`}>
          {saving ? '保存中...' : '保存'}
        </button>
        <button
          onClick={run}
          disabled={saving}
          className={`${BTN} bg-primary text-primary-foreground hover:bg-primary/90 ${saving ? 'opacity-50' : ''}`}
        >
          ▶ 运行
        </button>
      </div>
    </>
  );
}

const BTN =
  'rounded px-3 py-1 text-xs bg-secondary text-secondary-foreground hover:bg-secondary/80 transition-colors';
