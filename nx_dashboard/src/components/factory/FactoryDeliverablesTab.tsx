import { useMemo } from 'react';
import { useExecutionStore } from '@/stores/executionStore';
import { FileDiff } from 'lucide-react';

/** 交付物 diff 列表 MVP — 从最近完成的 stage_results 提取 */
export function FactoryDeliverablesTab() {
  const executions = useExecutionStore((s) => s.executions);

  const deliverables = useMemo(() => {
    const items: { executionId: string; stage: string; path: string; summary?: string }[] = [];
    for (const exec of executions) {
      if (exec.status !== 'completed' && exec.status !== 'running') continue;
      for (const stage of exec.stage_results ?? []) {
        for (const out of stage.outputs ?? []) {
          if (out.path || out.files_changed?.length) {
            items.push({
              executionId: exec.id,
              stage: stage.stage_name,
              path: out.path || out.files_changed?.join(', ') || '—',
              summary: out.summary,
            });
          }
        }
      }
    }
    return items.slice(0, 20);
  }, [executions]);

  if (deliverables.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
        <FileDiff className="w-12 h-12 mb-3 opacity-40" />
        <p className="text-sm">暂无交付物</p>
        <p className="text-xs mt-1">Run 完成后，变更文件会出现在这里</p>
      </div>
    );
  }

  return (
    <ul className="divide-y divide-border rounded-xl border border-border overflow-hidden">
      {deliverables.map((d, i) => (
        <li key={`${d.executionId}-${i}`} className="px-4 py-3 bg-card/30 hover:bg-accent/30">
          <div className="flex items-center gap-2">
            <FileDiff className="w-4 h-4 text-primary shrink-0" />
            <span className="text-sm font-medium truncate">{d.path}</span>
          </div>
          <p className="text-xs text-muted-foreground mt-1 ml-6">
            {d.stage} · Run {d.executionId.slice(0, 8)}
          </p>
          {d.summary && <p className="text-xs mt-1 ml-6 line-clamp-2">{d.summary}</p>}
        </li>
      ))}
    </ul>
  );
}
