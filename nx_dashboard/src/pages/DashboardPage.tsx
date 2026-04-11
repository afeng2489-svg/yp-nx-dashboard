import { useEffect, useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDashboardData } from '@/hooks/useReactQuery';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useSessionStore } from '@/stores/sessionStore';
import { Play, Clock, CheckCircle, XCircle, ChevronRight, Activity, Sparkles, Workflow as WorkflowIcon, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ExecutionTrendChart, ExecutionStatusPie, WorkflowPerformanceChart, ExecutionStatsSummary } from '@/components/charts';

// 数字滚动动画组件
function AnimatedNumber({ value }: { value: number }) {
  const [displayValue, setDisplayValue] = useState(0);
  const previousValue = useRef(value);
  const animationRef = useRef<number>();

  useEffect(() => {
    const start = previousValue.current;
    const end = value;
    const duration = 800;
    const startTime = performance.now();

    const animate = (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const easeOut = 1 - Math.pow(1 - progress, 3);
      const current = Math.round(start + (end - start) * easeOut);
      setDisplayValue(current);

      if (progress < 1) {
        animationRef.current = requestAnimationFrame(animate);
      }
    };

    animationRef.current = requestAnimationFrame(animate);
    previousValue.current = value;

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [value]);

  return <span>{displayValue}</span>;
}

// 渐变统计卡片组件
function StatCard({
  title,
  value,
  icon: Icon,
  gradient,
  delay = 0,
}: {
  title: string;
  value: number;
  icon: React.ComponentType<{ className?: string }>;
  gradient: string;
  delay?: number;
}) {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => setIsVisible(true), delay);
    return () => clearTimeout(timer);
  }, [delay]);

  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-2xl border border-border/50 bg-card transition-all duration-500 hover:shadow-lg hover:shadow-primary/5',
        isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'
      )}
    >
      {/* Background gradient */}
      <div className={cn('absolute inset-0 opacity-5', gradient)} />

      <div className="relative p-5">
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <p className="text-sm font-medium text-muted-foreground">{title}</p>
            <p className="text-3xl font-bold tracking-tight">
              <AnimatedNumber value={value} />
            </p>
          </div>
          <div
            className={cn(
              'p-3 rounded-xl transition-transform duration-500',
              isVisible ? 'scale-100 rotate-0' : 'scale-0 rotate-12'
            )}
            style={{ transitionDelay: `${delay + 200}ms` }}
          >
            <div className={cn('p-3 rounded-xl bg-gradient-to-br ', gradient, 'shadow-lg')}>
              <Icon className="w-6 h-6 text-white" />
            </div>
          </div>
        </div>

        {/* Bottom accent line */}
        <div className={cn('absolute bottom-0 left-0 right-0 h-0.5 opacity-30', gradient)} />
      </div>
    </div>
  );
}

// 活动会话项组件
function SessionItem({
  session,
  onClick,
}: {
  session: { id: string; workflow_id?: string; status: string; created_at: string };
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center justify-between p-4 rounded-xl transition-all duration-200',
        'bg-gradient-to-r from-card to-accent/30 border border-border/50',
        'hover:shadow-md hover:shadow-primary/5 hover:border-primary/20 hover:-translate-y-0.5',
        'text-left group'
      )}
    >
      <div className="flex items-center gap-4">
        <div className={cn(
          'w-10 h-10 rounded-xl flex items-center justify-center',
          'bg-gradient-to-br from-indigo-500/10 to-purple-500/10',
          'group-hover:from-indigo-500/20 group-hover:to-purple-500/20',
          'transition-all duration-200'
        )}>
          <Activity className="w-5 h-5 text-indigo-500" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium truncate group-hover:text-indigo-600 transition-colors">
            {session.workflow_id || '未指定工作流'}
          </p>
          <p className="text-sm text-muted-foreground">
            {new Date(session.created_at).toLocaleString('zh-CN', {
              month: 'short',
              day: 'numeric',
              hour: '2-digit',
              minute: '2-digit'
            })}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <span className={cn(
          'px-3 py-1.5 rounded-full text-xs font-medium',
          session.status === 'active'
            ? 'bg-emerald-500/10 text-emerald-600 border border-emerald-500/20'
            : 'bg-amber-500/10 text-amber-600 border border-amber-500/20'
        )}>
          {session.status === 'active' ? '活跃' : '空闲'}
        </span>
        <ChevronRight className="w-5 h-5 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all" />
      </div>
    </button>
  );
}

// 执行项组件
function ExecutionItem({
  execution,
  onClick,
}: {
  execution: { id: string; workflow_id: string; status: string; started_at?: string };
  onClick: () => void;
}) {
  const statusConfig: Record<string, { gradient: string; label: string; icon: React.ComponentType<{ className?: string }> }> = {
    pending: { gradient: 'from-slate-400 to-slate-500', label: '等待', icon: Clock },
    running: { gradient: 'from-blue-400 to-indigo-500', label: '运行', icon: Zap },
    completed: { gradient: 'from-emerald-400 to-green-500', label: '完成', icon: CheckCircle },
    failed: { gradient: 'from-red-400 to-rose-500', label: '失败', icon: XCircle },
    cancelled: { gradient: 'from-slate-400 to-gray-500', label: '取消', icon: XCircle },
  };

  const config = statusConfig[execution.status] || statusConfig.pending;
  const Icon = config.icon;

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center justify-between p-4 rounded-xl transition-all duration-200',
        'bg-gradient-to-r from-card to-accent/30 border border-border/50',
        'hover:shadow-md hover:shadow-primary/5 hover:border-primary/20 hover:-translate-y-0.5',
        'text-left group'
      )}
    >
      <div className="flex items-center gap-4">
        <div className={cn(
          'w-10 h-10 rounded-xl flex items-center justify-center',
          `bg-gradient-to-br ${config.gradient}`,
          'shadow-lg'
        )}>
          <Icon className="w-5 h-5 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium truncate group-hover:text-indigo-600 transition-colors">
            {execution.workflow_id}
          </p>
          <p className="text-sm text-muted-foreground">
            {execution.started_at
              ? new Date(execution.started_at).toLocaleString('zh-CN', {
                  month: 'short',
                  day: 'numeric',
                  hour: '2-digit',
                  minute: '2-digit'
                })
              : '未开始'}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <span className={cn(
          'px-3 py-1.5 rounded-full text-xs font-medium',
          'bg-gradient-to-r ' + config.gradient,
          'text-white shadow-md'
        )}>
          {config.label}
        </span>
        <ChevronRight className="w-5 h-5 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all" />
      </div>
    </button>
  );
}

export function DashboardPage() {
  const navigate = useNavigate();
  const { workflows } = useWorkflowStore();
  const { executions } = useExecutionStore();
  const { sessions } = useSessionStore();

  // Use React Query for parallel fetching with caching
  const { isLoading, refetch } = useDashboardData();

  // Combine loading states for initial load
  const isInitialLoading = isLoading;

  const runningCount = executions.filter((e) => e.status === 'running').length;
  const completedCount = executions.filter((e) => e.status === 'completed').length;
  const failedCount = executions.filter((e) => e.status === 'failed').length;
  const activeSessions = sessions.filter((s) => s.status === 'active' || s.status === 'idle' || s.status === 'pending');

  return (
    <div className="page-container space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              仪表盘
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">欢迎回来！查看您的工作流状态</p>
        </div>
        <button
          onClick={() => navigate('/workflows')}
          className="btn-primary"
        >
          <Sparkles className="w-4 h-4" />
          新建工作流
        </button>
      </div>

      {/* Loading State */}
      {isInitialLoading && workflows.length === 0 ? (
        <div className="space-y-6">
          {/* Skeleton for stat cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="h-32 rounded-2xl border border-border/50 bg-card animate-pulse">
                <div className="p-5 space-y-3">
                  <div className="h-4 w-20 bg-muted rounded" />
                  <div className="h-8 w-16 bg-muted rounded" />
                </div>
              </div>
            ))}
          </div>
          {/* Skeleton for charts */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="h-80 rounded-2xl border border-border/50 bg-card animate-pulse" />
            <div className="h-80 rounded-2xl border border-border/50 bg-card animate-pulse" />
          </div>
          <div className="h-64 rounded-2xl border border-border/50 bg-card animate-pulse" />
        </div>
      ) : (
      <>
      {/* 统计卡片 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 stagger-children">
        <StatCard
          title="工作流"
          value={workflows.length}
          icon={WorkflowIcon}
          gradient="from-indigo-500 via-purple-500 to-pink-500"
          delay={0}
        />
        <StatCard
          title="运行中"
          value={runningCount}
          icon={Zap}
          gradient="from-blue-500 via-indigo-500 to-purple-500"
          delay={100}
        />
        <StatCard
          title="已完成"
          value={completedCount}
          icon={CheckCircle}
          gradient="from-emerald-500 via-green-500 to-teal-500"
          delay={200}
        />
        <StatCard
          title="失败"
          value={failedCount}
          icon={XCircle}
          gradient="from-red-500 via-rose-500 to-pink-500"
          delay={300}
        />
      </div>

      {/* 图表区域 */}
      <div className="space-y-6">
        {/* Stats Summary */}
        <ExecutionStatsSummary />

        {/* Charts Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <ExecutionTrendChart />
          <ExecutionStatusPie />
        </div>

        {/* Workflow Performance */}
        <WorkflowPerformanceChart />
      </div>

      {/* 活动会话 */}
      <div className="bg-card rounded-2xl border border-border/50 p-6 shadow-sm">
        <div className="flex items-center justify-between mb-5">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10">
              <Activity className="w-5 h-5 text-indigo-500" />
            </div>
            <h2 className="text-lg font-semibold">活动会话</h2>
            <span className="px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-600 text-xs font-medium">
              {activeSessions.length}
            </span>
          </div>
          {activeSessions.length > 0 && (
            <button
              onClick={() => navigate('/sessions')}
              className="text-sm text-primary hover:text-primary/80 transition-colors font-medium"
            >
              查看全部 →
            </button>
          )}
        </div>
        {activeSessions.length === 0 ? (
          <div className="text-center py-8">
            <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-muted to-accent flex items-center justify-center">
              <Activity className="w-8 h-8 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">暂无活动会话</p>
            <p className="text-sm text-muted-foreground/70 mt-1">开始一个工作流来创建新会话</p>
          </div>
        ) : (
          <div className="space-y-3">
            {activeSessions.slice(0, 5).map((session) => (
              <SessionItem
                key={session.id}
                session={session}
                onClick={() => navigate(`/sessions?id=${session.id}`)}
              />
            ))}
          </div>
        )}
      </div>

      {/* 最近执行 */}
      <div className="bg-card rounded-2xl border border-border/50 p-6 shadow-sm">
        <div className="flex items-center justify-between mb-5">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-purple-500/10 to-pink-500/10">
              <Play className="w-5 h-5 text-purple-500" />
            </div>
            <h2 className="text-lg font-semibold">最近执行</h2>
          </div>
          {executions.length > 0 && (
            <button
              onClick={() => navigate('/executions')}
              className="text-sm text-primary hover:text-primary/80 transition-colors font-medium"
            >
              查看全部 →
            </button>
          )}
        </div>
        {executions.length === 0 ? (
          <div className="text-center py-8">
            <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-muted to-accent flex items-center justify-center">
              <Play className="w-8 h-8 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">暂无执行记录</p>
            <p className="text-sm text-muted-foreground/70 mt-1">执行工作流以查看执行历史</p>
          </div>
        ) : (
          <div className="space-y-3">
            {executions.slice(0, 5).map((execution) => (
              <ExecutionItem
                key={execution.id}
                execution={execution}
                onClick={() => navigate(`/executions`)}
              />
            ))}
          </div>
        )}
      </div>
      </>
      )}
    </div>
  );
}
