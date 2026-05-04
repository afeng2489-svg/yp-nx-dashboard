import { useState } from 'react';
import { useCanvasStore } from '@/stores/canvasStore';
import { api } from '@/api/client';
import { toast } from 'sonner';
import { TemplatesPanel } from './TemplatesPanel';

export function CanvasToolbar() {
  const { workflowName, workflowId, setWorkflowName, setWorkflowId, toYaml, loadFromYaml, resetExecStatus } =
    useCanvasStore();
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);

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
      toast.error('请先保存工作流');
      return;
    }
    setRunning(true);
    resetExecStatus();
    try {
      await api.executeWorkflow(workflowId);
      toast.success('已启动执行');
    } catch {
      toast.error('启动失败');
    } finally {
      setRunning(false);
    }
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
      <div className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950 px-4 py-2">
        <input
          className="rounded bg-zinc-800 px-2 py-1 text-sm text-zinc-200 border border-zinc-700 focus:outline-none w-48"
          value={workflowName}
          onChange={(e) => setWorkflowName(e.target.value)}
        />
        <div className="flex-1" />
        <button onClick={() => setShowTemplates(true)} className={BTN}>模板库</button>
        <button onClick={importFile} className={BTN}>导入 YAML</button>
        <button onClick={save} disabled={saving} className={`${BTN} ${saving ? 'opacity-50' : ''}`}>
          {saving ? '保存中...' : '保存'}
        </button>
        <button onClick={run} disabled={running} className={`${BTN} bg-blue-600 hover:bg-blue-500 ${running ? 'opacity-50' : ''}`}>
          {running ? '运行中...' : '▶ 运行'}
        </button>
      </div>
    </>
  );
}

const BTN = 'rounded px-3 py-1 text-xs text-zinc-200 bg-zinc-800 hover:bg-zinc-700 transition-colors';
