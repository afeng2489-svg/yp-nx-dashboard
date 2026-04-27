import { memo } from 'react';
import { Handle, Position, NodeProps, Node } from '@xyflow/react';
import { NODE_COLORS, NODE_ICONS, WorkflowNodeData } from './types';

type WorkflowNodeProps = NodeProps<Node<WorkflowNodeData>>;

function WorkflowNodeComponent({ data, selected }: WorkflowNodeProps) {
  const color = NODE_COLORS[data.type as keyof typeof NODE_COLORS] || NODE_COLORS.agent;
  const icon = NODE_ICONS[data.type as keyof typeof NODE_ICONS] || NODE_ICONS.agent;

  return (
    <div
      className={`
        px-4 py-3 rounded-lg border-2 min-w-[160px] transition-all
        shadow-md hover:shadow-lg
        ${selected ? 'ring-2 ring-primary ring-offset-2' : ''}
      `}
      style={{
        borderColor: color,
        backgroundColor: `color-mix(in srgb, ${color} 10%, white)`,
      }}
    >
      <div className="flex items-center gap-2">
        <span className="text-lg">{icon}</span>
        <div className="flex flex-col">
          <span className="font-medium text-sm">{data.label}</span>
          <span className="text-xs capitalize" style={{ color }}>
            {data.type}
          </span>
        </div>
      </div>

      {data.type === 'agent' && (
        <div className="mt-2 text-xs text-muted-foreground">
          {(data.config as { role: string }).role}
        </div>
      )}

      {data.type === 'stage' && (
        <div className="mt-2 text-xs text-muted-foreground">
          {(data.config as { parallel: boolean }).parallel ? 'Parallel' : 'Sequential'}
        </div>
      )}

      {data.type === 'condition' && (
        <div className="mt-2 text-xs text-muted-foreground truncate max-w-[140px]">
          {(data.config as { expression: string }).expression || 'No condition'}
        </div>
      )}

      {data.type === 'loop' && (
        <div className="mt-2 text-xs text-muted-foreground">
          Max: {(data.config as { maxIterations: number }).maxIterations}
        </div>
      )}

      <Handle type="target" position={Position.Top} className="!w-3 !h-3 !bg-primary border-2" />
      <Handle type="source" position={Position.Bottom} className="!w-3 !h-3 !bg-primary border-2" />
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-primary border-2"
        id="left"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-primary border-2"
        id="right"
      />
    </div>
  );
}

export const WorkflowNode = memo(WorkflowNodeComponent);
