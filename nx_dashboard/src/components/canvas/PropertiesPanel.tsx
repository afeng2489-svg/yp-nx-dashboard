import { useCanvasStore } from '@/stores/canvasStore';
import type { NodeData } from '@/stores/canvasStore';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

export function PropertiesPanel() {
  const { nodes, selectedNodeId, updateNodeData } = useCanvasStore();
  const node = nodes.find((n) => n.id === selectedNodeId);

  if (!node) {
    return (
      <div className="w-56 shrink-0 border-l border-border bg-card p-4 text-xs text-muted-foreground">
        选中节点后在此配置属性
      </div>
    );
  }

  const d = node.data;
  const upd = (patch: Partial<NodeData>) => updateNodeData(node.id, patch);

  return (
    <div className="w-56 shrink-0 border-l border-border bg-card p-3 overflow-y-auto text-xs">
      <p className="mb-3 font-semibold text-muted-foreground">属性</p>

      <Field label="名称">
        <input className={INPUT} value={d.label} onChange={(e) => upd({ label: e.target.value })} />
      </Field>

      {d.kind === 'agent' && (
        <>
          <Field label="模型">
            <input
              className={INPUT}
              value={d.model ?? ''}
              onChange={(e) => upd({ model: e.target.value })}
            />
          </Field>
          <Field label="System Prompt">
            <textarea
              className={`${INPUT} h-24 resize-none`}
              value={d.system_prompt ?? ''}
              onChange={(e) => upd({ system_prompt: e.target.value })}
            />
          </Field>
        </>
      )}

      {d.kind === 'shell' && (
        <>
          <Field label="命令">
            <input
              className={INPUT}
              value={d.command ?? ''}
              onChange={(e) => upd({ command: e.target.value })}
            />
          </Field>
          <Field label="超时(s)">
            <input
              className={INPUT}
              type="number"
              value={d.timeout ?? 30}
              onChange={(e) => upd({ timeout: Number(e.target.value) })}
            />
          </Field>
        </>
      )}

      {d.kind === 'quality_gate' && (
        <>
          <Field label="检查命令(每行一条)">
            <textarea
              className={`${INPUT} h-20 resize-none`}
              value={(d.checks ?? []).join('\n')}
              onChange={(e) => upd({ checks: e.target.value.split('\n').filter(Boolean) })}
            />
          </Field>
          <Field label="失败策略">
            <Select value={d.on_fail ?? 'retry'} onValueChange={(v) => upd({ on_fail: v })}>
              <SelectTrigger className={INPUT}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="retry">retry</SelectItem>
                <SelectItem value="continue">continue</SelectItem>
                <SelectItem value="fail">fail</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </>
      )}

      {d.kind === 'condition' && (
        <Field label="条件表达式">
          <input
            className={INPUT}
            value={d.condition ?? ''}
            onChange={(e) => upd({ condition: e.target.value })}
          />
        </Field>
      )}

      {d.kind === 'http' && (
        <>
          <Field label="Method">
            <Select value={d.method ?? 'GET'} onValueChange={(v) => upd({ method: v })}>
              <SelectTrigger className={INPUT}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {['GET', 'POST', 'PUT', 'DELETE', 'PATCH'].map((m) => (
                  <SelectItem key={m} value={m}>
                    {m}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="URL">
            <input
              className={INPUT}
              value={d.url ?? ''}
              onChange={(e) => upd({ url: e.target.value })}
            />
          </Field>
        </>
      )}

      {d.kind === 'approval' && (
        <>
          <Field label="审批问题">
            <input
              className={INPUT}
              value={d.question ?? ''}
              onChange={(e) => upd({ question: e.target.value })}
            />
          </Field>
          <Field label="选项(每行一条)">
            <textarea
              className={`${INPUT} h-16 resize-none`}
              value={(d.options ?? []).join('\n')}
              onChange={(e) => upd({ options: e.target.value.split('\n').filter(Boolean) })}
            />
          </Field>
        </>
      )}

      {d.kind === 'loop' && (
        <>
          <Field label="循环变量">
            <input
              className={INPUT}
              value={d.loop_var ?? ''}
              onChange={(e) => upd({ loop_var: e.target.value })}
            />
          </Field>
          <Field label="最大次数">
            <input
              className={INPUT}
              type="number"
              value={d.max_iterations ?? 10}
              onChange={(e) => upd({ max_iterations: Number(e.target.value) })}
            />
          </Field>
        </>
      )}
    </div>
  );
}

const INPUT =
  'w-full rounded bg-background px-2 py-1 text-xs border border-border focus:outline-none focus:border-primary';

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="mb-3">
      <p className="mb-1 text-muted-foreground">{label}</p>
      {children}
    </div>
  );
}
