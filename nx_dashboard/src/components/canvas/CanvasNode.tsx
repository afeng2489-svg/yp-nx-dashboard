import { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
import type { NodeData, NodeKind } from '@/stores/canvasStore';

const KIND_COLORS: Record<NodeKind, string> = {
  agent: 'border-blue-500 bg-blue-950/40',
  shell: 'border-zinc-500 bg-zinc-900/40',
  quality_gate: 'border-green-500 bg-green-950/40',
  condition: 'border-yellow-500 bg-yellow-950/40',
  http: 'border-purple-500 bg-purple-950/40',
  approval: 'border-orange-500 bg-orange-950/40',
  loop: 'border-cyan-500 bg-cyan-950/40',
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
};

export const CanvasNode = memo(({ data, selected }: NodeProps<NodeData>) => {
  const colorClass = KIND_COLORS[data.kind] ?? 'border-zinc-500';
  const ringClass = data.execStatus ? (STATUS_RING[data.execStatus] ?? '') : '';

  return (
    <div
      className={`min-w-[160px] rounded-lg border-2 px-3 py-2 text-sm text-white shadow-lg ${colorClass} ${ringClass} ${selected ? 'ring-2 ring-white/50' : ''}`}
    >
      <Handle type="target" position={Position.Top} className="!bg-zinc-400" />

      <div className="flex items-center gap-1.5 font-medium">
        <span>{KIND_ICONS[data.kind]}</span>
        <span className="truncate max-w-[120px]">{data.label}</span>
      </div>

      <div className="mt-1 text-xs text-zinc-400 truncate">
        {data.kind === 'agent' && data.model}
        {data.kind === 'shell' && data.command}
        {data.kind === 'quality_gate' && `${data.checks?.length ?? 0} 项检查`}
        {data.kind === 'condition' && data.condition}
        {data.kind === 'http' && `${data.method} ${data.url}`}
        {data.kind === 'approval' && data.question}
        {data.kind === 'loop' && `max ${data.max_iterations} 次`}
      </div>

      {data.execStatus === 'running' && data.execTokens != null && (
        <div className="mt-1 text-xs text-blue-300">{data.execTokens} tokens</div>
      )}
      {data.execStatus === 'failed' && data.execError && (
        <div className="mt-1 text-xs text-red-400 truncate" title={data.execError}>
          ❌ {data.execError}
        </div>
      )}
      {data.execDuration != null && data.execStatus !== 'running' && (
        <div className="mt-1 text-xs text-zinc-500">{(data.execDuration / 1000).toFixed(1)}s</div>
      )}

      <Handle type="source" position={Position.Bottom} className="!bg-zinc-400" />
      {data.kind === 'condition' && (
        <Handle type="source" position={Position.Right} id="false" className="!bg-red-400" />
      )}
    </div>
  );
});

CanvasNode.displayName = 'CanvasNode';
