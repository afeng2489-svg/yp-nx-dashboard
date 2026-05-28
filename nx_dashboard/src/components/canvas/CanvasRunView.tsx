import { useEffect, useMemo, useRef } from 'react';
import yaml from 'js-yaml';
import { ReactFlow, Background, Controls, MiniMap, type NodeTypes } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useCanvasStore } from '@/stores/canvasStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { CanvasNode } from './CanvasNode';
import { useCanvasExecution } from './useCanvasExecution';

const nodeTypes = { custom: CanvasNode } as NodeTypes;

interface CanvasRunViewProps {
  executionId: string;
  workflowId: string;
  className?: string;
}

/** 只读 Run Canvas — 阶段进度叠加 */
export function CanvasRunView({ executionId, workflowId, className }: CanvasRunViewProps) {
  const { nodes, edges, onNodesChange, onEdgesChange, loadFromYaml } = useCanvasStore();
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const loadedRef = useRef<string | null>(null);

  useCanvasExecution(executionId);

  useEffect(() => {
    void fetchWorkflows();
  }, [fetchWorkflows]);

  useEffect(() => {
    const key = `${workflowId}:${executionId}`;
    if (loadedRef.current === key) return;
    const wf = workflows.find((w) => w.id === workflowId);
    if (!wf) return;
    loadedRef.current = key;
    try {
      const defn = (wf as { definition?: Record<string, unknown> }).definition;
      const yamlStr = yaml.dump(defn && Object.keys(defn).length > 0 ? defn : wf);
      loadFromYaml(yamlStr);
    } catch {
      /* ignore */
    }
  }, [workflowId, executionId, workflows, loadFromYaml]);

  const hasNodes = useMemo(() => nodes.length > 0, [nodes.length]);

  if (!hasNodes) {
    return (
      <div className={`flex items-center justify-center min-h-[200px] text-sm text-muted-foreground ${className ?? ''}`}>
        加载产线 Canvas…
      </div>
    );
  }

  return (
    <div className={`h-[280px] rounded-lg border border-border/50 bg-zinc-950 ${className ?? ''}`}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={16} color="#333" />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable />
      </ReactFlow>
    </div>
  );
}
