import { useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  Panel,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useEditorStore, NodeType } from '@/stores/editorStore';
import { WorkflowNode } from './WorkflowNode';

const nodeTypes = {
  workflowNode: WorkflowNode,
};

export function WorkflowCanvas() {
  const {
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNode,
    selectNode,
  } = useEditorStore();

  const { screenToFlowPosition } = useReactFlow();

  const handleDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const type = event.dataTransfer.getData('application/reactflow') as NodeType;
      if (!type) return;

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      addNode(type, position);
    },
    [addNode, screenToFlowPosition]
  );

  const handleDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const handlePaneClick = useCallback(() => {
    selectNode(null);
  }, [selectNode]);

  return (
    <div
      style={{ position: 'absolute', inset: 0 }}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
    >
      <ReactFlow
        style={{ width: '100%', height: '100%' }}
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onPaneClick={handlePaneClick}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid
        snapGrid={[16, 16]}
        defaultEdgeOptions={{
          animated: true,
          style: { strokeWidth: 2 },
        }}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color="hsl(var(--border))"
        />
        <Controls
          className="!bg-card !border-border shadow-md"
          showInteractive={false}
        />
        <MiniMap
          className="!bg-card !border-border shadow-md"
          nodeColor={(node) => {
            switch (node.data?.type) {
              case 'agent':
                return 'hsl(221.2 83.2% 53.3%)';
              case 'stage':
                return 'hsl(142 76% 36%)';
              case 'condition':
                return 'hsl(38 92% 50%)';
              case 'loop':
                return 'hsl(280 65% 60%)';
              default:
                return 'hsl(var(--muted))';
            }
          }}
          maskColor="hsl(var(--background) / 0.8)"
        />

        <Panel position="top-left" className="!m-4">
          <div className="bg-card border border-border rounded-lg shadow-md px-3 py-2 text-sm text-muted-foreground">
            拖拽左侧节点到画布
          </div>
        </Panel>
      </ReactFlow>
    </div>
  );
}
