import type { NodeKind } from '@/stores/canvasStore';
import { useCanvasStore } from '@/stores/canvasStore';

const GROUPS: { label: string; kinds: NodeKind[] }[] = [
  { label: 'AI', kinds: ['agent'] },
  { label: '执行', kinds: ['shell', 'http'] },
  { label: '控制流', kinds: ['condition', 'loop', 'approval'] },
  { label: '质量', kinds: ['quality_gate'] },
  { label: '编排', kinds: ['workflow'] },
];

const KIND_LABELS: Record<NodeKind, string> = {
  agent: '🤖 AI 调用',
  shell: '⚙️ 代码执行',
  quality_gate: '✅ 质量门',
  condition: '🔀 条件分支',
  http: '🌐 HTTP 请求',
  approval: '👤 人工审批',
  loop: '🔁 循环',
  workflow: '🔗 工作流',
};

export function NodePanel() {
  const addNode = useCanvasStore((s) => s.addNode);

  const onDragStart = (e: React.DragEvent, kind: NodeKind) => {
    e.dataTransfer.setData('nodeKind', kind);
    e.dataTransfer.effectAllowed = 'move';
  };

  return (
    <div className="w-48 shrink-0 border-r border-border bg-card p-3 overflow-y-auto">
      <p className="mb-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider">节点</p>
      {GROUPS.map((g) => (
        <div key={g.label} className="mb-4">
          <p className="mb-1 text-xs text-muted-foreground/60">{g.label}</p>
          {g.kinds.map((kind) => (
            <div
              key={kind}
              draggable
              onDragStart={(e) => onDragStart(e, kind)}
              onClick={() => addNode(kind, { x: 200 + Math.random() * 200, y: 100 + Math.random() * 200 })}
              className="mb-1 cursor-grab rounded px-2 py-1.5 text-xs hover:bg-accent transition-colors active:cursor-grabbing select-none"
            >
              {KIND_LABELS[kind]}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
