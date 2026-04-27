import { useEffect, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { WorkflowCanvas } from '@/components/editor/WorkflowCanvas';
import { NodePalette } from '@/components/editor/NodePalette';
import { PropertyPanel } from '@/components/editor/PropertyPanel';
import { CommandPalette } from '@/components/editor/CommandPalette';
import { TemplateLibrary, workflowTemplates } from '@/components/editor/templates';
import { useEditorStore } from '@/stores/editorStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useNavigate } from 'react-router-dom';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { showSuccess, showError } from '@/lib/toast';

export function EditorPage() {
  const navigate = useNavigate();
  const { workflowName, setWorkflowName, selectedNodeId, isDirty, loadedWorkflowId, exportWorkflow, clearCanvas } = useEditorStore();
  const { createWorkflow, updateWorkflow } = useWorkflowStore();
  const [saving, setSaving] = useState(false);
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  // Convert editor format to API workflow format
  const convertToApiFormat = () => {
    const { nodes, edges, name, id } = exportWorkflow();

    // Extract agents and stages from nodes
    const agents: { id: string; role: string; model: string; prompt: string; depends_on: string[] }[] = [];
    const stages: { name: string; agents: string[]; parallel: boolean }[] = [];
    const nodeIdToRole: Record<string, string> = {};

    nodes.forEach(node => {
      if (node.data.type === 'agent') {
        const config = node.data.config as { role: string; model: string; prompt: string };
        const agentId = node.id;
        nodeIdToRole[node.id] = config.role;
        agents.push({
          id: agentId,
          role: config.role,
          model: config.model,
          prompt: config.prompt,
          depends_on: edges
            .filter(e => e.target === node.id)
            .map(e => nodeIdToRole[e.source])
            .filter(Boolean),
        });
      } else if (node.data.type === 'stage') {
        const config = node.data.config as { name: string; parallel: boolean; agents: string[] };
        const stageAgents = nodes
          .filter(n => n.data.type === 'agent' && edges.some(e => e.source === node.id && e.target === n.id))
          .map(n => (n.data.config as { role: string }).role);
        stages.push({
          name: config.name,
          agents: stageAgents,
          parallel: config.parallel,
        });
      }
    });

    return {
      id,
      name,
      version: '1.0',
      description: '',
      definition: {
        stages,
        agents,
      },
    };
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const workflow = convertToApiFormat();
      if (loadedWorkflowId) {
        await updateWorkflow(loadedWorkflowId, workflow);
      } else {
        await createWorkflow(workflow);
      }
      showSuccess('工作流已保存!');
      clearCanvas();
      navigate('/workflows');
    } catch (error) {
      showError('保存失败: ' + (error as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const handleBack = () => {
    if (isDirty) {
      showConfirm(
        '离开页面',
        '有未保存的更改，确定要离开吗？',
        () => navigate('/workflows'),
        'warning'
      );
      return;
    }
    navigate('/workflows');
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const target = e.target as HTMLElement;
        if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') {
          return;
        }
        if (selectedNodeId) {
          useEditorStore.getState().deleteNode(selectedNodeId);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedNodeId]);

  return (
    <ReactFlowProvider>
      <div className="h-screen bg-background" style={{ display: 'grid', gridTemplateRows: 'auto 1fr auto' }}>
        <header className="flex items-center justify-between px-4 py-3 border-b border-border bg-card">
          <div className="flex items-center gap-4">
            <button
              onClick={handleBack}
              className="flex items-center gap-1 px-2 py-1 text-sm rounded-md hover:bg-accent transition-colors"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
              返回
            </button>
            <input
              type="text"
              value={workflowName}
              onChange={(e) => setWorkflowName(e.target.value)}
              className="text-lg font-semibold bg-transparent outline-none border-b-2 border-transparent focus:border-primary transition-colors"
            />
            {isDirty && <span className="px-2 py-0.5 text-xs bg-yellow-500 text-white rounded">未保存</span>}
            {!isDirty && loadedWorkflowId && <span className="px-2 py-0.5 text-xs bg-green-500 text-white rounded">已保存</span>}
          </div>

          <div className="flex items-center gap-2">
            <TemplateLibrary templates={workflowTemplates} />

            <button
              onClick={() => useEditorStore.getState().setCommandPaletteOpen(true)}
              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg border border-border hover:bg-accent transition-colors"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
              </svg>
              <span className="hidden sm:inline">Command</span>
              <kbd className="hidden sm:inline px-1.5 py-0.5 text-xs bg-muted rounded border border-border">⌘K</kbd>
            </button>

            <button
              onClick={handleSave}
              disabled={saving}
              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
              </svg>
              {saving ? '保存中...' : '保存'}
            </button>

            <button
              onClick={() => {
                const workflow = convertToApiFormat();
                const blob = new Blob([JSON.stringify(workflow, null, 2)], { type: 'application/json' });
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `${workflow.name.replace(/\s+/g, '_')}.json`;
                a.click();
                URL.revokeObjectURL(url);
              }}
              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg border border-border hover:bg-accent transition-colors"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
              </svg>
              Export
            </button>
          </div>
        </header>

        <div className="flex overflow-hidden" style={{ minHeight: 0 }}>
          <aside className="w-auto p-4 border-r border-border overflow-y-auto" style={{ flexShrink: 0 }}>
            <NodePalette />
          </aside>

          <main className="relative" style={{ flex: '1 1 0', minWidth: 0, overflow: 'hidden' }}>
            <WorkflowCanvas />
          </main>

          <aside className="w-auto p-4 border-l border-border overflow-y-auto" style={{ flexShrink: 0 }}>
            <PropertyPanel />
          </aside>
        </div>

        <footer className="px-4 py-2 border-t border-border bg-card">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <div className="flex items-center gap-4">
              <span>节点: {useEditorStore.getState().nodes.length}</span>
              <span>连线: {useEditorStore.getState().edges.length}</span>
            </div>
            <div className="flex items-center gap-4">
              <span>按 Ctrl+K 打开命令</span>
              <span>拖拽节点到画布</span>
            </div>
          </div>
        </footer>
      </div>

      <CommandPalette templates={workflowTemplates} />

      {/* Confirm Modal */}
      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={() => {
          confirmState.onConfirm();
          hideConfirm();
        }}
        onCancel={hideConfirm}
        variant={confirmState.variant || 'danger'}
      />
    </ReactFlowProvider>
  );
}