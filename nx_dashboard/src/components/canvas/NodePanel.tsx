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

// 通过 Agent prompt 模板驱动的节点（非引擎原生）
const AI_ASSISTED: NodeKind[] = ['shell', 'http', 'workflow'];

const KIND_TIPS: Partial<Record<NodeKind, string>> = {
  shell: 'AI 通过 Bash 工具执行命令',
  http: 'AI 通过 Fetch 工具发起请求',
  workflow: 'AI 调用 API 启动子工作流',
  condition: '融合到上游 stage 的条件跳转',
  quality_gate: '融合到下游 stage 的质量门',
};

export function NodePanel() {
  const addNode = useCanvasStore((s) => s.addNode);

  const onDragStart = (e: React.DragEvent, kind: NodeKind) => {
    e.dataTransfer.setData('nodeKind', kind);
    e.dataTransfer.effectAllowed = 'move';
  };

  return (
    <div className="w-52 shrink-0 border-r border-border bg-card p-3 overflow-y-auto">
      <p className="mb-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
        节点
      </p>
      {GROUPS.map((g) => (
        <div key={g.label} className="mb-4">
          <p className="mb-1 text-xs text-muted-foreground/60">{g.label}</p>
          {g.kinds.map((kind) => {
            const tip = KIND_TIPS[kind];
            const isAIAssisted = AI_ASSISTED.includes(kind);
            return (
              <div
                key={kind}
                draggable
                onDragStart={(e) => onDragStart(e, kind)}
                onClick={() =>
                  addNode(kind, { x: 200 + Math.random() * 200, y: 100 + Math.random() * 200 })
                }
                title={tip || KIND_LABELS[kind]}
                className="mb-1 cursor-grab rounded px-2 py-1.5 text-xs hover:bg-accent transition-colors active:cursor-grabbing select-none flex items-center justify-between gap-1"
              >
                <span>{KIND_LABELS[kind]}</span>
                {isAIAssisted && (
                  <span className="text-[9px] text-amber-600 bg-amber-500/10 px-1 py-0.5 rounded border border-amber-500/20">
                    AI辅助
                  </span>
                )}
              </div>
            );
          })}
        </div>
      ))}
      <div className="mt-4 p-2 rounded bg-amber-500/5 border border-amber-500/20 text-[10px] text-muted-foreground leading-relaxed">
        <span className="text-amber-600 font-medium">ⓘ AI 辅助</span> 节点由 Claude 调用 Bash/Fetch
        工具执行，适合轻量任务。高频/高性能场景请在 <b>AI 调用</b> 节点的 prompt 里直接编写。
      </div>
    </div>
  );
}
