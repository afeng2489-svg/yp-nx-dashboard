import { useMemo } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  Legend,
} from 'recharts';
import { useExecutionStore } from '@/stores/executionStore';
import { cn } from '@/lib/utils';
import { TrendingUp, PieChart as PieIcon, BarChart2, Activity } from 'lucide-react';

const COLORS = {
  completed: '#10b981',
  failed: '#f43f5e',
  running: '#6366f1',
  pending: '#94a3b8',
  cancelled: '#f59e0b',
};

interface ChartCardProps {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  badge?: string;
}

function ChartCard({ title, icon, children, className, badge }: ChartCardProps) {
  return (
    <div className={cn('bg-card rounded-2xl border border-border/40 overflow-hidden', className)}>
      <div className="flex items-center justify-between px-5 pt-4 pb-3 border-b border-border/30">
        <div className="flex items-center gap-2">
          <div className="text-muted-foreground">{icon}</div>
          <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        </div>
        {badge && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground font-medium">
            {badge}
          </span>
        )}
      </div>
      <div className="p-5">{children}</div>
    </div>
  );
}

const tooltipStyle = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '10px',
  fontSize: 12,
  boxShadow: '0 4px 16px rgba(0,0,0,0.12)',
};

export function ExecutionTrendChart({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    const last7Days = Array.from({ length: 7 }, (_, i) => {
      const date = new Date();
      date.setDate(date.getDate() - (6 - i));
      return date.toISOString().split('T')[0];
    });

    return last7Days.map((date) => {
      const dayExecs = executions.filter((e) => e.started_at?.split('T')[0] === date);
      return {
        displayDate: new Date(date + 'T00:00:00').toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }),
        completed: dayExecs.filter((e) => e.status === 'completed').length,
        failed: dayExecs.filter((e) => e.status === 'failed').length,
      };
    });
  }, [executions]);

  const total = data.reduce((s, d) => s + d.completed + d.failed, 0);

  return (
    <ChartCard title="执行趋势" icon={<TrendingUp size={15} />} badge={`近7天 · ${total}次`} className={className}>
      <div className="h-56">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 4, right: 4, left: -24, bottom: 0 }}>
            <defs>
              <linearGradient id="gradCompleted" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={COLORS.completed} stopOpacity={0.25} />
                <stop offset="100%" stopColor={COLORS.completed} stopOpacity={0} />
              </linearGradient>
              <linearGradient id="gradFailed" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={COLORS.failed} stopOpacity={0.2} />
                <stop offset="100%" stopColor={COLORS.failed} stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" strokeOpacity={0.5} />
            <XAxis dataKey="displayDate" tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} axisLine={false} tickLine={false} />
            <YAxis tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} axisLine={false} tickLine={false} allowDecimals={false} />
            <Tooltip contentStyle={tooltipStyle} />
            <Legend iconType="circle" iconSize={8} wrapperStyle={{ fontSize: 12, paddingTop: 8 }} />
            <Area type="monotone" dataKey="completed" stroke={COLORS.completed} strokeWidth={2} fill="url(#gradCompleted)" name="完成" dot={{ r: 3, fill: COLORS.completed }} activeDot={{ r: 5 }} />
            <Area type="monotone" dataKey="failed" stroke={COLORS.failed} strokeWidth={2} fill="url(#gradFailed)" name="失败" dot={{ r: 3, fill: COLORS.failed }} activeDot={{ r: 5 }} />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

const RADIAN = Math.PI / 180;
function renderCustomLabel({ cx, cy, midAngle, innerRadius, outerRadius, percent }: { cx: number; cy: number; midAngle: number; innerRadius: number; outerRadius: number; percent: number }) {
  if (percent < 0.08) return null;
  const r = innerRadius + (outerRadius - innerRadius) * 0.5;
  const x = cx + r * Math.cos(-midAngle * RADIAN);
  const y = cy + r * Math.sin(-midAngle * RADIAN);
  return (
    <text x={x} y={y} fill="white" textAnchor="middle" dominantBaseline="central" fontSize={11} fontWeight={600}>
      {`${(percent * 100).toFixed(0)}%`}
    </text>
  );
}

export function ExecutionStatusPie({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    const counts = executions.reduce((acc, e) => {
      acc[e.status] = (acc[e.status] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    return [
      { name: '完成', value: counts.completed || 0, color: COLORS.completed },
      { name: '失败', value: counts.failed || 0, color: COLORS.failed },
      { name: '运行中', value: counts.running || 0, color: COLORS.running },
      { name: '等待', value: counts.pending || 0, color: COLORS.pending },
      { name: '取消', value: counts.cancelled || 0, color: COLORS.cancelled },
    ].filter((d) => d.value > 0);
  }, [executions]);

  const total = data.reduce((s, d) => s + d.value, 0);

  if (data.length === 0) {
    return (
      <ChartCard title="状态分布" icon={<PieIcon size={15} />} className={className}>
        <div className="h-56 flex items-center justify-center text-muted-foreground text-sm">暂无数据</div>
      </ChartCard>
    );
  }

  return (
    <ChartCard title="状态分布" icon={<PieIcon size={15} />} badge={`共 ${total} 次`} className={className}>
      <div className="flex items-center gap-4">
        <div className="h-48 flex-1 min-w-0">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie data={data} cx="50%" cy="50%" innerRadius={52} outerRadius={80} paddingAngle={3} dataKey="value" labelLine={false} label={renderCustomLabel}>
                {data.map((entry, i) => (
                  <Cell key={i} fill={entry.color} stroke="transparent" />
                ))}
              </Pie>
              <Tooltip contentStyle={tooltipStyle} />
            </PieChart>
          </ResponsiveContainer>
        </div>
        <div className="flex flex-col gap-2.5 shrink-0">
          {data.map((entry) => (
            <div key={entry.name} className="flex items-center gap-2">
              <div className="w-2.5 h-2.5 rounded-sm shrink-0" style={{ backgroundColor: entry.color }} />
              <span className="text-xs text-muted-foreground w-12">{entry.name}</span>
              <span className="text-xs font-semibold tabular-nums">{entry.value}</span>
            </div>
          ))}
        </div>
      </div>
    </ChartCard>
  );
}

export function WorkflowPerformanceChart({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    const stats = executions.reduce((acc, e) => {
      const wid = e.workflow_id;
      if (!acc[wid]) acc[wid] = { total: 0, completed: 0 };
      acc[wid].total += 1;
      if (e.status === 'completed') acc[wid].completed += 1;
      return acc;
    }, {} as Record<string, { total: number; completed: number }>);

    return Object.entries(stats)
      .map(([id, s]) => ({
        name: id.length > 10 ? id.slice(0, 10) + '…' : id,
        rate: s.total > 0 ? Math.round((s.completed / s.total) * 100) : 0,
        total: s.total,
      }))
      .sort((a, b) => b.total - a.total)
      .slice(0, 6);
  }, [executions]);

  if (data.length === 0) {
    return (
      <ChartCard title="工作流成功率" icon={<BarChart2 size={15} />} className={className}>
        <div className="h-48 flex items-center justify-center text-muted-foreground text-sm">暂无数据</div>
      </ChartCard>
    );
  }

  return (
    <ChartCard title="工作流成功率" icon={<BarChart2 size={15} />} className={className}>
      <div className="h-52">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data} margin={{ top: 4, right: 4, left: -24, bottom: 0 }} barSize={28}>
            <defs>
              <linearGradient id="barGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={COLORS.completed} stopOpacity={1} />
                <stop offset="100%" stopColor="#059669" stopOpacity={0.8} />
              </linearGradient>
              <linearGradient id="barGradLow" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={COLORS.failed} stopOpacity={1} />
                <stop offset="100%" stopColor="#e11d48" stopOpacity={0.8} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" strokeOpacity={0.5} vertical={false} />
            <XAxis dataKey="name" tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} axisLine={false} tickLine={false} />
            <YAxis tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} axisLine={false} tickLine={false} domain={[0, 100]} tickFormatter={(v) => `${v}%`} />
            <Tooltip contentStyle={tooltipStyle} formatter={(v: number) => [`${v}%`, '成功率']} />
            <Bar dataKey="rate" radius={[6, 6, 0, 0]} name="成功率">
              {data.map((entry, i) => (
                <Cell key={i} fill={entry.rate >= 70 ? 'url(#barGrad)' : 'url(#barGradLow)'} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

export function ExecutionStatsSummary({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const stats = useMemo(() => {
    const total = executions.length;
    const completed = executions.filter((e) => e.status === 'completed').length;
    const failed = executions.filter((e) => e.status === 'failed').length;
    const running = executions.filter((e) => e.status === 'running').length;
    return {
      total,
      successRate: total > 0 ? Math.round((completed / total) * 100) : 0,
      failed,
      running,
    };
  }, [executions]);

  const items = [
    { label: '总执行', value: stats.total, icon: <Activity size={14} />, gradient: 'from-violet-500/10 to-purple-500/5', accent: 'text-violet-500', border: 'border-violet-500/20' },
    { label: '成功率', value: `${stats.successRate}%`, icon: <TrendingUp size={14} />, gradient: 'from-emerald-500/10 to-green-500/5', accent: 'text-emerald-500', border: 'border-emerald-500/20' },
    { label: '失败', value: stats.failed, icon: <BarChart2 size={14} />, gradient: 'from-rose-500/10 to-red-500/5', accent: 'text-rose-500', border: 'border-rose-500/20' },
    { label: '运行中', value: stats.running, icon: <PieIcon size={14} />, gradient: 'from-indigo-500/10 to-blue-500/5', accent: 'text-indigo-500', border: 'border-indigo-500/20' },
  ];

  return (
    <div className={cn('grid grid-cols-2 lg:grid-cols-4 gap-3', className)}>
      {items.map((item) => (
        <div key={item.label} className={cn('bg-card rounded-xl border p-4 relative overflow-hidden', item.border)}>
          <div className={cn('absolute inset-0 bg-gradient-to-br', item.gradient)} />
          <div className="relative">
            <div className={cn('mb-2', item.accent)}>{item.icon}</div>
            <p className="text-2xl font-bold tabular-nums">{item.value}</p>
            <p className="text-xs text-muted-foreground mt-0.5">{item.label}</p>
          </div>
        </div>
      ))}
    </div>
  );
}
