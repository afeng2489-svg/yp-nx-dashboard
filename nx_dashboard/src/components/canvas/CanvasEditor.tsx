import { useCallback, useRef } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type NodeTypes,
  type ReactFlowInstance,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useCanvasStore } from '@/stores/canvasStore';
import type { NodeKind } from '@/stores/canvasStore';
import { CanvasNode } from './CanvasNode';
import { NodePanel } from './NodePanel';
import { PropertiesPanel } from './PropertiesPanel';
import { YamlPanel } from './YamlPanel';
import { CanvasToolbar } from './CanvasToolbar';

const nodeTypes = { custom: CanvasNode } as NodeTypes;

export function CanvasEditor() {
  const { nodes, edges, onNodesChange, onEdgesChange, onConnect, setSelectedNode, addNode } =
    useCanvasStore();
  const rfRef = useRef<ReactFlowInstance | null>(null);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const kind = e.dataTransfer.getData('nodeKind') as NodeKind;
      if (!kind || !rfRef.current) return;
      const bounds = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const position = rfRef.current.screenToFlowPosition({
        x: e.clientX - bounds.left,
        y: e.clientY - bounds.top,
      });
      addNode(kind, position);
    },
    [addNode],
  );

  return (
    <div className="flex h-full flex-col bg-zinc-950">
      <CanvasToolbar />
      <div className="flex flex-1 overflow-hidden">
        <NodePanel />
        <div className="flex-1" onDrop={onDrop} onDragOver={(e) => e.preventDefault()}>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={(_, node) => setSelectedNode(node.id)}
            onPaneClick={() => setSelectedNode(null)}
            onInit={(instance) => {
              rfRef.current = instance as unknown as ReactFlowInstance;
            }}
            fitView
            className="bg-zinc-900"
          >
            <Background color="#3f3f46" gap={20} />
            <Controls className="!bg-zinc-800 !border-zinc-700" />
            <MiniMap className="!bg-zinc-800" nodeColor="#3f3f46" />
          </ReactFlow>
        </div>
        <PropertiesPanel />
        <YamlPanel />
      </div>
    </div>
  );
}
