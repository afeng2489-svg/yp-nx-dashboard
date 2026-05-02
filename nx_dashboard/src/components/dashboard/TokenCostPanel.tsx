import { useMemo } from 'react';
import { useExecutionStore } from '@/stores/executionStore';
import { Coins, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function formatCost(usd: number): string {
  if (usd >= 1) return `$${usd.toFixed(2)}`;
  if (usd >= 0.01) return `$${usd.toFixed(3)}`;
  if (usd > 0) return `$${usd.toFixed(4)}`;
  return '$0.00';
}

export function TokenCostSummary() {
  const { executions } = useExecutionStore();

  const totals = useMemo(() => {
    let totalTokens = 0;
    let totalCost = 0;

    for (const e of executions) {
      totalTokens += e.total_tokens ?? 0;
      totalCost += e.total_cost_usd ?? 0;
    }

    return { totalTokens, totalCost };
  }, [executions]);

  const runningTokens = useMemo(() => {
    let tokens = 0;
    let cost = 0;
    for (const e of executions) {
      if (e.status === 'running' || e.status === 'paused') {
        tokens += e.total_tokens ?? 0;
        cost += e.total_cost_usd ?? 0;
      }
    }
    return { tokens, cost };
  }, [executions]);

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
      <div className="bg-card rounded-xl border border-border/50 p-4 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-indigo-500 to-purple-500 opacity-5" />
        <p className="text-xs text-muted-foreground mb-1">总 Token</p>
        <p className="text-2xl font-bold">{formatTokens(totals.totalTokens)}</p>
      </div>
      <div className="bg-card rounded-xl border border-border/50 p-4 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-emerald-500 to-green-500 opacity-5" />
        <p className="text-xs text-muted-foreground mb-1">总费用</p>
        <p className="text-2xl font-bold">{formatCost(totals.totalCost)}</p>
      </div>
      <div className="bg-card rounded-xl border border-border/50 p-4 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-blue-500 to-indigo-500 opacity-5" />
        <p className="text-xs text-muted-foreground mb-1">本次 Token</p>
        <p className="text-2xl font-bold">
          {runningTokens.tokens > 0 ? formatTokens(runningTokens.tokens) : '-'}
        </p>
      </div>
      <div className="bg-card rounded-xl border border-border/50 p-4 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-amber-500 to-orange-500 opacity-5" />
        <p className="text-xs text-muted-foreground mb-1">本次费用</p>
        <p className="text-2xl font-bold">
          {runningTokens.cost > 0 ? formatCost(runningTokens.cost) : '-'}
        </p>
      </div>
    </div>
  );
}

export function ExecutionTokenBadge({ executionId }: { executionId: string }) {
  const executions = useExecutionStore((s) => s.executions);
  const execution = executions.find((e) => e.id === executionId);

  if (!execution || (!execution.total_tokens && !execution.total_cost_usd)) {
    return null;
  }

  const tokens = execution.total_tokens ?? 0;
  const cost = execution.total_cost_usd ?? 0;

  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          'flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs',
          'bg-indigo-500/10 text-indigo-600 border border-indigo-500/20',
        )}
      >
        <Zap className="w-3 h-3" />
        <span>{formatTokens(tokens)}</span>
      </div>
      <div
        className={cn(
          'flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs',
          'bg-emerald-500/10 text-emerald-600 border border-emerald-500/20',
        )}
      >
        <Coins className="w-3 h-3" />
        <span>{formatCost(cost)}</span>
      </div>
    </div>
  );
}
