import { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
import type { NodeData, NodeKind } from '@/stores/canvasStore';

const KIND_COLORS: Record<NodeKind, string> = {
  agent: 'border-blue-500 bg-blue-500/10',
  shell: 'border-border bg-card',
  quality_gate: 'border-green-500 bg-green-500/10',
  condition: 'border-yellow-500 bg-yellow-500/10',
  http: 'border-purple-500 bg-purple-500/10',
  approval: 'border-orange-500 bg-orange-500/10',
  loop: 'border-cyan-500 bg-cyan-500/10',
  workflow: 'border-primary bg-primary/10',
};

const STATUS_RING: Record<string, string> = {
  running: 'ring-2 ring-blue-400 animate-pulse',
  success: 'ring-2 ring-green-400',
  failed: 'ring-2 ring-red-500',
  retrying: 'ring-2 ring-orange-400',
};

const KIND_ICONS: Record<NodeKind, string> = {
  agent: '🤖',
  shell: '⚙️',
  quality_gate: '✅',
  condition: '🔀',
  http: '🌐',
  approval: '👤',
  loop: '🔁',
  workflow: '🔗',
};

export const CanvasNode = memo(({ data: rawData, selected }: NodeProps) => {
  const data = rawData as NodeData;
  const colorClass = KIND_COLORS[data.kind] ?? 'border-zinc-500';
  const ringClass = data.execStatus ? (STATUS_RING[data.execStatus] ?? '') : '';

  return (
    <div
      className={`min-w-[160px] rounded-lg border-2 px-3 py-2 text-sm shadow-lg ${colorClass} ${ringClass} ${selected ? 'ring-2 ring-primary/50' : ''}`}
    >
      <Handle type="target" position={Position.Top} className="!bg-border" />

      <div className="flex items-center gap-1.5 font-medium">
        <span>{KIND_ICONS[data.kind]}</span>
        <span className="truncate max-w-[120px]">{data.label}</span>
      </div>

      <div className="mt-1 text-xs text-muted-foreground truncate">
        {data.kind === 'agent' && data.model}
        {data.kind === 'shell' && data.command}
        {data.kind === 'quality_gate' && `${data.checks?.length ?? 0} 项检查`}
        {data.kind === 'condition' && data.condition}
        {data.kind === 'http' && `${data.method} ${data.url}`}
        {data.kind === 'approval' && data.question}
        {data.kind === 'loop' && `max ${data.max_iterations} 次`}
      </div>

      {data.execStatus === 'running' && data.execTokens != null && (
        <div className="mt-1 text-xs text-blue-500">{data.execTokens} tokens</div>
      )}
      {data.execStatus === 'failed' && data.execError && (
        <div className="mt-1 text-xs text-destructive truncate" title={data.execError}>
          ❌ {data.execError}
        </div>
      )}
      {data.execDuration != null && data.execStatus !== 'running' && (
        <div className="mt-1 text-xs text-muted-foreground/60">
          {(data.execDuration / 1000).toFixed(1)}s
        </div>
      )}

      <Handle type="source" position={Position.Bottom} className="!bg-border" />
      {data.kind === 'condition' && (
        <Handle type="source" position={Position.Right} id="false" className="!bg-red-400" />
      )}
    </div>
  );
});

CanvasNode.displayName = 'CanvasNode';
