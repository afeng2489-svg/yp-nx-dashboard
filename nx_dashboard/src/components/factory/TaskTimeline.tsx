import { useMemo } from 'react';
import { CheckCircle2, Circle, Loader2, ShieldAlert } from 'lucide-react';
import type { Execution } from '@/stores/executionStore';
import { cn } from '@/lib/utils';

export interface TaskTimelineProps {
  execution: Execution;
  userIntent?: string;
}

/** AF-UX-10：本任务时间线 — 用户输入 / 阶段 / 审批 */
export function TaskTimeline({ execution, userIntent }: TaskTimelineProps) {
  const events = useMemo(() => {
    const items: { id: string; label: string; detail?: string; state: 'done' | 'active' | 'pending' | 'failed' }[] = [];

    const prompt =
      userIntent ||
      (typeof execution.variables?.task === 'string' ? execution.variables.task : '') ||
      (typeof execution.variables?.prompt === 'string' ? execution.variables.prompt : '');

    if (prompt) {
      items.push({ id: 'intent', label: '用户输入', detail: prompt.slice(0, 80), state: 'done' });
    }

    for (const sr of execution.stage_results ?? []) {
      if (sr.stage_name.startsWith('agent:')) continue;
      items.push({
        id: sr.stage_name,
        label: sr.stage_name,
        state: 'done',
      });
    }

    if (execution.current_stage && !items.some((i) => i.id === execution.current_stage)) {
      items.push({
        id: execution.current_stage,
        label: execution.current_stage,
        state: execution.status === 'failed' ? 'failed' : 'active',
      });
    }

    for (const ev of execution.approval_events ?? []) {
      items.push({
        id: `approval-${ev.stage_name}-${ev.decided_at}`,
        label: ev.approved ? `批准 · ${ev.stage_name}` : `驳回 · ${ev.stage_name}`,
        state: 'done',
      });
    }

    return items;
  }, [execution, userIntent]);

  if (events.length === 0) return null;

  return (
    <section className="space-y-2" data-testid="task-timeline">
      <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">本任务时间线</h4>
      <ol className="space-y-1.5">
        {events.map((ev) => {
          const Icon =
            ev.state === 'done'
              ? CheckCircle2
              : ev.state === 'failed'
                ? ShieldAlert
                : ev.state === 'active'
                  ? Loader2
                  : Circle;
          return (
            <li key={ev.id} className="flex items-start gap-2 text-xs">
              <Icon
                className={cn(
                  'h-3.5 w-3.5 mt-0.5 shrink-0',
                  ev.state === 'done' && 'text-emerald-500',
                  ev.state === 'active' && 'text-primary animate-spin',
                  ev.state === 'failed' && 'text-destructive',
                  ev.state === 'pending' && 'text-muted-foreground',
                )}
              />
              <div className="min-w-0">
                <p className="font-medium text-foreground">{ev.label}</p>
                {ev.detail && <p className="text-muted-foreground truncate">{ev.detail}</p>}
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
