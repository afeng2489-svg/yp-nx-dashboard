import { useMemo } from 'react';
import { useExecutionStore, Execution } from '@/stores/executionStore';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { cn } from '@/lib/utils';

function estimateStageDurations(execution: Execution): { name: string; duration: number }[] {
  const stages = execution.stage_results ?? [];
  if (stages.length === 0) return [];

  const results: { name: string; duration: number }[] = [];

  for (let i = 0; i < stages.length; i++) {
    const stage = stages[i];
    const completedAt = stage.completed_at;
    if (!completedAt) continue;

    let start: Date;
    if (i === 0) {
      start = execution.started_at ? new Date(execution.started_at) : new Date(completedAt);
    } else {
      const prevCompleted = stages[i - 1].completed_at;
      start = prevCompleted ? new Date(prevCompleted) : new Date(completedAt);
    }

    const end = new Date(completedAt);
    const durationSec = Math.max(0, Math.round((end.getTime() - start.getTime()) / 1000));
    results.push({ name: stage.stage_name, duration: durationSec });
  }

  return results;
}

function StageDurationChart() {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    const runningExec = executions.find((e) => e.status === 'running');
    const target =
      runningExec ?? executions.find((e) => e.stage_results && e.stage_results.length > 0);
    if (!target) return [];

    return estimateStageDurations(target).map((d) => ({
      ...d,
      display:
        d.duration >= 60 ? `${Math.floor(d.duration / 60)}m ${d.duration % 60}s` : `${d.duration}s`,
    }));
  }, [executions]);

  if (data.length === 0) return null;

  const maxDuration = Math.max(...data.map((d) => d.duration), 1);

  return (
    <div className="bg-card rounded-2xl border border-border/50 p-5 shadow-sm">
      <h3 className="text-sm font-medium text-muted-foreground mb-3">阶段耗时</h3>
      <div className="h-48">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data} margin={{ top: 5, right: 10, left: -20, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis dataKey="name" tick={{ fontSize: 11 }} stroke="hsl(var(--muted-foreground))" />
            <YAxis
              tick={{ fontSize: 11 }}
              stroke="hsl(var(--muted-foreground))"
              tickFormatter={(v) => `${v}s`}
            />
            <Tooltip
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: 12,
              }}
              formatter={(value: number) => [`${value}s`, '耗时']}
            />
            <Bar dataKey="duration" radius={[4, 4, 0, 0]} name="耗时(s)">
              {data.map((entry, index) => {
                const ratio = entry.duration / maxDuration;
                const color = ratio > 0.8 ? '#ef4444' : ratio > 0.5 ? '#f59e0b' : '#22c55e';
                return <Cell key={`cell-${index}`} fill={color} />;
              })}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function StageProgressList() {
  const { executions } = useExecutionStore();

  const activeExecutions = useMemo(
    () => executions.filter((e) => e.status === 'running' || e.status === 'paused'),
    [executions],
  );

  if (activeExecutions.length === 0) return null;

  return (
    <div className="bg-card rounded-2xl border border-border/50 p-5 shadow-sm">
      <h3 className="text-sm font-medium text-muted-foreground mb-3">当前阶段</h3>
      <div className="space-y-3">
        {activeExecutions.map((exec) => {
          const stages = exec.stage_results ?? [];
          const total = stages.length;
          return (
            <div key={exec.id} className="space-y-1.5">
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground font-mono truncate">
                  {exec.id.slice(0, 8)}...
                </span>
                <span className="font-medium">{total} 阶段完成</span>
              </div>
              <div className="h-2 rounded-full bg-muted overflow-hidden flex">
                {stages.map((stage, idx) => {
                  const qg = stage.quality_gate_result;
                  const passed = qg ? qg.passed : true;
                  return (
                    <div
                      key={stage.stage_name}
                      className={cn(
                        'h-full',
                        passed ? 'bg-emerald-500' : 'bg-red-500',
                        idx === 0 ? 'rounded-l-full' : '',
                        idx === stages.length - 1 ? 'rounded-r-full' : '',
                      )}
                      style={{ flex: 1 }}
                      title={`${stage.stage_name}: ${passed ? 'PASS' : 'FAIL'}`}
                    />
                  );
                })}
              </div>
              {stages.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {stages.slice(-3).map((stage) => (
                    <span
                      key={stage.stage_name}
                      className={cn(
                        'text-[10px] px-1.5 py-0.5 rounded',
                        (stage.quality_gate_result?.passed ?? true)
                          ? 'bg-emerald-500/10 text-emerald-600'
                          : 'bg-red-500/10 text-red-600',
                      )}
                    >
                      {stage.stage_name}
                    </span>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function StageMetricsPanel() {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <StageDurationChart />
      <StageProgressList />
    </div>
  );
}
