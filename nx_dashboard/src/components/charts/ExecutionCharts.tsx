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
} from 'recharts';
import { useExecutionStore } from '@/stores/executionStore';
import { cn } from '@/lib/utils';

// Chart colors
const COLORS = {
  completed: '#22c55e',
  failed: '#ef4444',
  running: '#3b82f6',
  pending: '#94a3b8',
  cancelled: '#f59e0b',
};

interface ChartCardProps {
  title: string;
  children: React.ReactNode;
  className?: string;
}

function ChartCard({ title, children, className }: ChartCardProps) {
  return (
    <div className={cn('bg-card rounded-2xl border border-border/50 p-5', className)}>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">{title}</h3>
      {children}
    </div>
  );
}

// Execution trend chart (area chart)
export function ExecutionTrendChart({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    // Group executions by day for the last 7 days
    const last7Days = Array.from({ length: 7 }, (_, i) => {
      const date = new Date();
      date.setDate(date.getDate() - (6 - i));
      return date.toISOString().split('T')[0];
    });

    const counts = last7Days.map((date) => {
      const dayExecutions = executions.filter((e) => {
        const execDate = e.started_at?.split('T')[0];
        return execDate === date;
      });

      return {
        date,
        displayDate: new Date(date).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }),
        completed: dayExecutions.filter((e) => e.status === 'completed').length,
        failed: dayExecutions.filter((e) => e.status === 'failed').length,
        running: dayExecutions.filter((e) => e.status === 'running').length,
      };
    });

    return counts;
  }, [executions]);

  return (
    <ChartCard title="执行趋势（近7天）" className={className}>
      <div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <defs>
              <linearGradient id="colorCompleted" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor={COLORS.completed} stopOpacity={0.3} />
                <stop offset="95%" stopColor={COLORS.completed} stopOpacity={0} />
              </linearGradient>
              <linearGradient id="colorFailed" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor={COLORS.failed} stopOpacity={0.3} />
                <stop offset="95%" stopColor={COLORS.failed} stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis
              dataKey="displayDate"
              tick={{ fontSize: 12 }}
              stroke="hsl(var(--muted-foreground))"
            />
            <YAxis
              tick={{ fontSize: 12 }}
              stroke="hsl(var(--muted-foreground))"
              allowDecimals={false}
            />
            <Tooltip
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: 12,
              }}
            />
            <Area
              type="monotone"
              dataKey="completed"
              stroke={COLORS.completed}
              fillOpacity={1}
              fill="url(#colorCompleted)"
              name="完成"
            />
            <Area
              type="monotone"
              dataKey="failed"
              stroke={COLORS.failed}
              fillOpacity={1}
              fill="url(#colorFailed)"
              name="失败"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

// Execution status distribution (pie chart)
export function ExecutionStatusPie({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    const statusCounts = executions.reduce(
      (acc, e) => {
        acc[e.status] = (acc[e.status] || 0) + 1;
        return acc;
      },
      {} as Record<string, number>,
    );

    return [
      { name: '完成', value: statusCounts.completed || 0, color: COLORS.completed },
      { name: '失败', value: statusCounts.failed || 0, color: COLORS.failed },
      { name: '运行中', value: statusCounts.running || 0, color: COLORS.running },
      { name: '等待', value: statusCounts.pending || 0, color: COLORS.pending },
      { name: '取消', value: statusCounts.cancelled || 0, color: COLORS.cancelled },
    ].filter((d) => d.value > 0);
  }, [executions]);

  if (data.length === 0) {
    return (
      <ChartCard title="执行状态分布" className={className}>
        <div className="h-48 flex items-center justify-center text-muted-foreground text-sm">
          暂无执行数据
        </div>
      </ChartCard>
    );
  }

  return (
    <ChartCard title="执行状态分布" className={className}>
      <div className="h-48">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={data}
              cx="50%"
              cy="50%"
              innerRadius={50}
              outerRadius={70}
              paddingAngle={2}
              dataKey="value"
            >
              {data.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={entry.color} />
              ))}
            </Pie>
            <Tooltip
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: 12,
              }}
            />
          </PieChart>
        </ResponsiveContainer>
      </div>
      {/* Legend */}
      <div className="flex flex-wrap gap-3 mt-2 justify-center">
        {data.map((entry) => (
          <div key={entry.name} className="flex items-center gap-1.5 text-xs">
            <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: entry.color }} />
            <span className="text-muted-foreground">{entry.name}</span>
            <span className="font-medium">{entry.value}</span>
          </div>
        ))}
      </div>
    </ChartCard>
  );
}

// Workflow performance chart (bar chart)
export function WorkflowPerformanceChart({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const data = useMemo(() => {
    // Group executions by workflow and calculate success rate
    const workflowStats = executions.reduce(
      (acc, e) => {
        const wid = e.workflow_id;
        if (!acc[wid]) {
          acc[wid] = { total: 0, completed: 0, failed: 0 };
        }
        acc[wid].total += 1;
        if (e.status === 'completed') acc[wid].completed += 1;
        if (e.status === 'failed') acc[wid].failed += 1;
        return acc;
      },
      {} as Record<string, { total: number; completed: number; failed: number }>,
    );

    return Object.entries(workflowStats)
      .map(([workflow_id, stats]) => ({
        workflow_id: workflow_id.length > 12 ? workflow_id.slice(0, 12) + '...' : workflow_id,
        successRate: stats.total > 0 ? Math.round((stats.completed / stats.total) * 100) : 0,
        total: stats.total,
      }))
      .sort((a, b) => b.total - a.total)
      .slice(0, 6);
  }, [executions]);

  if (data.length === 0) {
    return (
      <ChartCard title="工作流成功率" className={className}>
        <div className="h-48 flex items-center justify-center text-muted-foreground text-sm">
          暂无执行数据
        </div>
      </ChartCard>
    );
  }

  return (
    <ChartCard title="工作流成功率" className={className}>
      <div className="h-48">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis
              dataKey="workflow_id"
              tick={{ fontSize: 11 }}
              stroke="hsl(var(--muted-foreground))"
            />
            <YAxis
              tick={{ fontSize: 11 }}
              stroke="hsl(var(--muted-foreground))"
              domain={[0, 100]}
              tickFormatter={(v) => `${v}%`}
            />
            <Tooltip
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: 12,
              }}
              formatter={(value: number) => [`${value}%`, '成功率']}
            />
            <Bar
              dataKey="successRate"
              fill={COLORS.completed}
              radius={[4, 4, 0, 0]}
              name="成功率"
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </ChartCard>
  );
}

// Stats summary with mini charts
export function ExecutionStatsSummary({ className }: { className?: string }) {
  const { executions } = useExecutionStore();

  const stats = useMemo(() => {
    const total = executions.length;
    const completed = executions.filter((e) => e.status === 'completed').length;
    const failed = executions.filter((e) => e.status === 'failed').length;
    const running = executions.filter((e) => e.status === 'running').length;
    const avgDuration =
      executions
        .filter((e) => e.started_at && e.finished_at)
        .reduce((acc, e) => {
          const start = new Date(e.started_at!).getTime();
          const end = new Date(e.finished_at!).getTime();
          return acc + (end - start);
        }, 0) / (completed || 1);

    return {
      total,
      completed,
      failed,
      running,
      successRate: total > 0 ? Math.round((completed / total) * 100) : 0,
      avgDuration: Math.round(avgDuration / 1000), // seconds
    };
  }, [executions]);

  const statItems = [
    { label: '总执行', value: stats.total, color: 'from-indigo-500 to-purple-500' },
    { label: '成功率', value: `${stats.successRate}%`, color: 'from-emerald-500 to-green-500' },
    { label: '失败', value: stats.failed, color: 'from-red-500 to-rose-500' },
    { label: '运行中', value: stats.running, color: 'from-blue-500 to-indigo-500' },
  ];

  return (
    <div className={cn('grid grid-cols-2 lg:grid-cols-4 gap-3', className)}>
      {statItems.map((item) => (
        <div
          key={item.label}
          className="bg-card rounded-xl border border-border/50 p-4 relative overflow-hidden"
        >
          <div className={`absolute inset-0 bg-gradient-to-br ${item.color} opacity-5`} />
          <p className="text-xs text-muted-foreground mb-1">{item.label}</p>
          <p className="text-2xl font-bold">{item.value}</p>
        </div>
      ))}
    </div>
  );
}
