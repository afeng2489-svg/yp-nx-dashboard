import { useEffect, useState } from 'react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { DollarSign, Coins, Activity, Zap, Route } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { api, type RoutingRule } from '@/api/client';

interface CostSummary {
  total_tokens: number;
  total_cost_usd: number;
  total_executions: number;
}

interface DailyCost {
  date: string;
  tokens: number;
  cost_usd: number;
}

interface WorkflowCost {
  workflow_id: string;
  total_tokens: number;
  total_cost_usd: number;
  execution_count: number;
}

// 各模型相对成本（每百万 token，USD，仅用于估算对比）
const MODEL_COST_PER_MT: Record<string, number> = {
  'claude-opus-4-7': 15,
  'claude-sonnet-4-6': 3,
  'claude-haiku-4-5-20251001': 0.25,
  'qwen-2.5-72b': 0.5,
  'mimo-v2.5-pro': 1.0,
  'ollama/llama3': 0,
};

function getModelCost(model: string): number {
  const key = Object.keys(MODEL_COST_PER_MT).find((k) => model.includes(k));
  return key ? MODEL_COST_PER_MT[key] : 2;
}

function ModelRoutingCostSection() {
  const [rules, setRules] = useState<RoutingRule[]>([]);

  useEffect(() => {
    api
      .listRoutingRules()
      .then(setRules)
      .catch(() => {});
  }, []);

  const enabledRules = rules.filter((r) => r.enabled);
  if (enabledRules.length === 0) return null;

  // 按模型聚合规则数量
  const byModel = enabledRules.reduce<Record<string, number>>((acc, r) => {
    acc[r.model] = (acc[r.model] ?? 0) + 1;
    return acc;
  }, {});

  const chartData = Object.entries(byModel).map(([model, count]) => ({
    model: model.length > 20 ? `…${model.slice(-18)}` : model,
    fullModel: model,
    rules: count,
    cost_per_mt: getModelCost(model),
  }));

  return (
    <div className="rounded-xl border bg-card p-4">
      <div className="flex items-center gap-2 mb-4">
        <Route className="w-5 h-5 text-primary" />
        <h2 className="text-lg font-semibold">模型路由成本对比</h2>
        <span className="text-xs text-muted-foreground ml-1">（基于当前路由规则估算）</span>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm mb-4">
          <thead>
            <tr className="border-b text-muted-foreground">
              <th className="text-left py-2 px-3">模型</th>
              <th className="text-right py-2 px-3">路由规则数</th>
              <th className="text-right py-2 px-3">参考价格 ($/MT)</th>
              <th className="text-right py-2 px-3">相对成本</th>
            </tr>
          </thead>
          <tbody>
            {chartData.map((row) => (
              <tr key={row.fullModel} className="border-b last:border-0">
                <td className="py-2 px-3 font-mono text-xs">{row.fullModel}</td>
                <td className="text-right py-2 px-3">{row.rules}</td>
                <td className="text-right py-2 px-3">
                  {row.cost_per_mt === 0 ? '免费（本地）' : `$${row.cost_per_mt}`}
                </td>
                <td className="text-right py-2 px-3">
                  <div className="flex items-center justify-end gap-2">
                    <div
                      className="h-2 rounded-full bg-primary/60"
                      style={{
                        width: `${Math.min(100, (row.cost_per_mt / 15) * 100)}px`,
                      }}
                    />
                    <span className="text-xs text-muted-foreground w-8">
                      {row.cost_per_mt === 0
                        ? '0%'
                        : `${Math.round((row.cost_per_mt / 15) * 100)}%`}
                    </span>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ResponsiveContainer width="100%" height={200}>
        <BarChart data={chartData} layout="vertical">
          <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
          <XAxis type="number" tick={{ fontSize: 11 }} tickFormatter={(v: number) => `$${v}`} />
          <YAxis type="category" dataKey="model" tick={{ fontSize: 10 }} width={120} />
          <Tooltip
            contentStyle={{
              background: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '8px',
            }}
            formatter={(v: number) => [`$${v}/MT`, '参考价格']}
          />
          <Bar dataKey="cost_per_mt" fill="#6366f1" radius={[0, 4, 4, 0]} name="参考价格 ($/MT)" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

type Range = 7 | 30 | 90;

export function CostPage() {
  const [summary, setSummary] = useState<CostSummary>({
    total_tokens: 0,
    total_cost_usd: 0,
    total_executions: 0,
  });
  const [daily, setDaily] = useState<DailyCost[]>([]);
  const [workflows, setWorkflows] = useState<WorkflowCost[]>([]);
  const [range, setRange] = useState<Range>(30);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchAll = async () => {
      setLoading(true);
      try {
        const [sumRes, dayRes, wfRes] = await Promise.all([
          fetch(`${API_BASE_URL}/api/v1/costs/summary`),
          fetch(`${API_BASE_URL}/api/v1/costs/by-day?days=${range}`),
          fetch(`${API_BASE_URL}/api/v1/costs/by-workflow`),
        ]);
        if (sumRes.ok) setSummary(await sumRes.json());
        if (dayRes.ok) {
          const dayData = await dayRes.json();
          setDaily(dayData.data ?? []);
        }
        if (wfRes.ok) {
          const wfData = await wfRes.json();
          setWorkflows(wfData.workflows ?? []);
        }
      } catch (err) {
        console.error('Failed to load cost data', err);
      } finally {
        setLoading(false);
      }
    };
    void fetchAll();
  }, [range]);

  const fmt = (n: number) =>
    n >= 1_000_000
      ? `${(n / 1_000_000).toFixed(2)}M`
      : n >= 1000
        ? `${(n / 1000).toFixed(1)}K`
        : n.toFixed(0);

  const fmtUsd = (n: number) => (n >= 1 ? `$${n.toFixed(2)}` : `$${(n * 100).toFixed(1)}¢`);

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">成本看板</h1>
        <div className="flex gap-1 rounded-lg bg-muted p-1">
          {[7, 30, 90].map((d) => (
            <button
              key={d}
              onClick={() => setRange(d as Range)}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                range === d
                  ? 'bg-background shadow-sm text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {d}天
            </button>
          ))}
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={<DollarSign className="w-5 h-5" />}
          label="总花费"
          value={fmtUsd(summary.total_cost_usd)}
          color="text-rose-500"
        />
        <StatCard
          icon={<Coins className="w-5 h-5" />}
          label="总 Token"
          value={fmt(summary.total_tokens)}
          color="text-amber-500"
        />
        <StatCard
          icon={<Activity className="w-5 h-5" />}
          label="总执行数"
          value={String(summary.total_executions)}
          color="text-blue-500"
        />
        <StatCard
          icon={<Zap className="w-5 h-5" />}
          label="平均 Token/次"
          value={fmt(
            summary.total_executions > 0 ? summary.total_tokens / summary.total_executions : 0,
          )}
          color="text-emerald-500"
        />
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-48 text-muted-foreground">加载中...</div>
      ) : (
        <>
          {/* Daily Trend */}
          <div className="rounded-xl border bg-card p-4">
            <h2 className="text-lg font-semibold mb-4">每日 Token / Cost 趋势</h2>
            {daily.length === 0 ? (
              <EmptyState />
            ) : (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={daily}>
                  <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
                  <XAxis
                    dataKey="date"
                    tick={{ fontSize: 12 }}
                    tickFormatter={(v: string) => v.slice(5)}
                  />
                  <YAxis
                    yAxisId="tokens"
                    orientation="left"
                    tick={{ fontSize: 12 }}
                    tickFormatter={fmt}
                  />
                  <YAxis
                    yAxisId="cost"
                    orientation="right"
                    tick={{ fontSize: 12 }}
                    tickFormatter={(v: number) => `$${v.toFixed(2)}`}
                  />
                  <Tooltip
                    contentStyle={{
                      background: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                  />
                  <Legend />
                  <Line
                    yAxisId="tokens"
                    type="monotone"
                    dataKey="tokens"
                    stroke="#f59e0b"
                    strokeWidth={2}
                    dot={false}
                    name="Token"
                  />
                  <Line
                    yAxisId="cost"
                    type="monotone"
                    dataKey="cost_usd"
                    stroke="#ef4444"
                    strokeWidth={2}
                    dot={false}
                    name="Cost (USD)"
                  />
                </LineChart>
              </ResponsiveContainer>
            )}
          </div>

          {/* By Workflow */}
          <div className="rounded-xl border bg-card p-4">
            <h2 className="text-lg font-semibold mb-4">按工作流 Cost 排行</h2>
            {workflows.length === 0 ? (
              <EmptyState />
            ) : (
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={workflows.slice(0, 10)}>
                  <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
                  <XAxis
                    dataKey="workflow_id"
                    tick={{ fontSize: 11 }}
                    tickFormatter={(v: string) => (v.length > 12 ? `${v.slice(0, 12)}…` : v)}
                  />
                  <YAxis
                    tick={{ fontSize: 12 }}
                    tickFormatter={(v: number) => `$${v.toFixed(2)}`}
                  />
                  <Tooltip
                    contentStyle={{
                      background: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                    }}
                  />
                  <Legend />
                  <Bar
                    dataKey="total_cost_usd"
                    fill="#6366f1"
                    radius={[4, 4, 0, 0]}
                    name="Cost (USD)"
                  />
                </BarChart>
              </ResponsiveContainer>
            )}
          </div>

          {/* Top 5 Table */}
          {workflows.length > 0 && (
            <div className="rounded-xl border bg-card p-4">
              <h2 className="text-lg font-semibold mb-4">Top 5 最贵工作流</h2>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-muted-foreground">
                      <th className="text-left py-2 px-3">工作流</th>
                      <th className="text-right py-2 px-3">Token</th>
                      <th className="text-right py-2 px-3">Cost</th>
                      <th className="text-right py-2 px-3">执行次数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {workflows.slice(0, 5).map((wf) => (
                      <tr key={wf.workflow_id} className="border-b last:border-0">
                        <td className="py-2 px-3 font-mono text-xs">{wf.workflow_id}</td>
                        <td className="text-right py-2 px-3">{fmt(wf.total_tokens)}</td>
                        <td className="text-right py-2 px-3">{fmtUsd(wf.total_cost_usd)}</td>
                        <td className="text-right py-2 px-3">{wf.execution_count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
          {/* Model Routing Cost Comparison */}
          <ModelRoutingCostSection />
        </>
      )}
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  color,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="rounded-xl border bg-card p-4 flex items-center gap-4">
      <div className={`p-2 rounded-lg bg-muted ${color}`}>{icon}</div>
      <div>
        <p className="text-sm text-muted-foreground">{label}</p>
        <p className="text-xl font-bold">{value}</p>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex items-center justify-center h-48 text-muted-foreground text-sm">
      暂无数据 — 执行工作流后将在此显示
    </div>
  );
}
