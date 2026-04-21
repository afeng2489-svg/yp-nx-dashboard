import { useEffect, useState, useCallback, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { useExecutionsQuery } from '@/hooks/useReactQuery';
import { useExecutionStore, Execution, StageResult } from '@/stores/executionStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { XCircle, Clock, CheckCircle, Play, AlertCircle, Loader2, X, ChevronRight, Terminal, Activity, PauseCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

// 工作流操作说明
const WORKFLOW_OPERATIONS = [
  { key: '1', action: '创建', desc: '点击"新建工作流"进入编辑器' },
  { key: '2', action: '编辑', desc: '从列表点击编辑图标，或在画布上拖拽节点' },
  { key: '3', action: '保存', desc: '点击"保存"按钮保存到后端' },
  { key: '4', action: '执行', desc: '点击播放图标执行工作流' },
  { key: '5', action: '导入/导出', desc: '使用 Export 按钮导出 JSON，可用于备份或分享' },
];

// 状态配置
const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    gradient: 'from-slate-400 to-gray-500',
    label: '等待中',
  },
  running: {
    icon: Loader2,
    gradient: 'from-blue-500 to-indigo-500',
    label: '运行中',
  },
  paused: {
    icon: PauseCircle,
    gradient: 'from-amber-400 to-orange-500',
    label: '等待输入',
  },
  completed: {
    icon: CheckCircle,
    gradient: 'from-emerald-500 to-green-500',
    label: '已完成',
  },
  failed: {
    icon: XCircle,
    gradient: 'from-red-500 to-rose-500',
    label: '失败',
  },
  cancelled: {
    icon: XCircle,
    gradient: 'from-slate-400 to-gray-500',
    label: '已取消',
  },
} as const;

// 计算执行持续时间
function formatDuration(startedAt?: string, finishedAt?: string): string {
  if (!startedAt) return '-';
  const start = new Date(startedAt).getTime();
  const end = finishedAt ? new Date(finishedAt).getTime() : Date.now();
  const duration = Math.floor((end - start) / 1000);

  if (duration < 60) return `${duration}秒`;
  if (duration < 3600) return `${Math.floor(duration / 60)}分${duration % 60}秒`;
  return `${Math.floor(duration / 3600)}小时${Math.floor((duration % 3600) / 60)}分`;
}

// 格式化时间
function formatTime(dateStr?: string): string {
  if (!dateStr) return '-';
  return new Date(dateStr).toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  });
}

// 执行详情弹窗
function ExecutionDetailModal({
  execution,
  onClose,
  onCancel,
}: {
  execution: Execution;
  onClose: () => void;
  onCancel: (id: string) => void;
}) {
  const [activeTab, setActiveTab] = useState<'stages' | 'logs'>('stages');
  const [expandedStages, setExpandedStages] = useState<Set<string>>(new Set());

  const config = STATUS_CONFIG[execution.status];
  const Icon = config.icon;

  const toggleStage = (stageName: string) => {
    setExpandedStages((prev) => {
      const next = new Set(prev);
      if (next.has(stageName)) {
        next.delete(stageName);
      } else {
        next.add(stageName);
      }
      return next;
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-gradient-to-b from-black/50 to-black/70 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col animate-scale-in border border-border/50 overflow-hidden">
        {/* 弹窗头部 */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <div className="flex items-center gap-4">
            <div className={cn('p-2.5 rounded-xl bg-gradient-to-br ', config.gradient, 'shadow-lg')}>
              <Icon className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">执行详情</h2>
              <p className="text-sm text-muted-foreground font-mono">ID: {execution.id}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {execution.status === 'running' && (
              <button
                onClick={() => onCancel(execution.id)}
                className="px-4 py-2 text-sm rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white shadow-lg shadow-red-500/25 hover:shadow-red-500/40 transition-all"
              >
                取消执行
              </button>
            )}
            {execution.status === 'paused' && (
              <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30">
                <PauseCircle className="w-4 h-4 text-amber-500 animate-pulse" />
                <span className="text-sm text-amber-600 font-medium">等待用户输入</span>
              </div>
            )}
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* 执行信息 */}
        <div className="px-6 py-4 bg-gradient-to-r from-indigo-500/5 to-purple-500/5 border-b border-border/50">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">工作流</p>
              <p className="font-semibold truncate">{execution.workflow_id}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">状态</p>
              <span className={cn(
                'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium',
                'bg-gradient-to-r ' + config.gradient,
                'text-white shadow-md'
              )}>
                {config.label}
              </span>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">开始时间</p>
              <p className="font-medium text-sm">{formatTime(execution.started_at)}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">持续时间</p>
              <p className="font-medium text-sm">{formatDuration(execution.started_at, execution.finished_at)}</p>
            </div>
          </div>
        </div>

        {/* Tab 切换 */}
        <div className="flex border-b border-border/50">
          <button
            onClick={() => setActiveTab('stages')}
            className={cn(
              'flex-1 px-4 py-3 text-sm font-medium transition-all relative',
              activeTab === 'stages'
                ? 'text-indigo-600'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            阶段结果
            {activeTab === 'stages' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-indigo-500 to-purple-500" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('logs')}
            className={cn(
              'flex-1 px-4 py-3 text-sm font-medium transition-all relative',
              activeTab === 'logs'
                ? 'text-indigo-600'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            执行日志
            {activeTab === 'logs' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-indigo-500 to-purple-500" />
            )}
          </button>
        </div>

        {/* 内容区域 */}
        <div className="flex-1 overflow-auto p-6">
          {activeTab === 'stages' ? (
            <div className="space-y-3">
              {(!execution.stage_results || execution.stage_results.length === 0) ? (
                <div className="text-center py-12">
                  <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
                    <Activity className="w-8 h-8 text-indigo-500" />
                  </div>
                  <p className="text-muted-foreground">暂无阶段数据</p>
                </div>
              ) : (
                execution.stage_results?.map((result, idx) => (
                  <StageResultCard
                    key={idx}
                    result={result}
                    isExpanded={expandedStages.has(result.stage_name)}
                    onToggle={() => toggleStage(result.stage_name)}
                  />
                ))
              )}
            </div>
          ) : (
            <ExecutionLogs executionId={execution.id} />
          )}
        </div>

        {/* 错误信息 */}
        {execution.error && (
          <div className="px-6 py-4 border-t border-border/50 bg-gradient-to-r from-red-500/5 to-rose-500/5">
            <div className="flex items-start gap-3 p-3 rounded-xl bg-red-500/10 border border-red-500/20">
              <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
              <div>
                <p className="font-medium text-red-600">执行错误</p>
                <p className="text-sm mt-1 text-red-600/80">{execution.error}</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// 阶段结果卡片
function StageResultCard({
  result,
  isExpanded,
  onToggle,
}: {
  result: StageResult;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="border border-border/50 rounded-xl overflow-hidden bg-gradient-to-r from-card to-accent/20">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 hover:from-indigo-500/5 hover:to-purple-500/5 transition-all"
      >
        <div className="flex items-center gap-3">
          <div className={cn(
            'p-1.5 rounded-lg transition-transform duration-200',
            isExpanded ? 'bg-emerald-500/10 rotate-90' : 'bg-indigo-500/10'
          )}>
            <ChevronRight className={cn('w-4 h-4 text-indigo-500 transition-transform', isExpanded && 'rotate-90')} />
          </div>
          <CheckCircle className="w-4 h-4 text-emerald-500" />
          <span className="font-medium">{result.stage_name}</span>
        </div>
        <span className="text-sm text-muted-foreground">
          {result.completed_at ? formatTime(result.completed_at) : '进行中'}
        </span>
      </button>
      {isExpanded && result.outputs && result.outputs.length > 0 && (
        <div className="px-4 pb-4 space-y-2">
          {result.outputs.map((output, idx) => (
            <div key={idx} className="p-3 bg-[#1e1e1e] rounded-lg text-xs text-gray-300 font-mono overflow-auto border border-white/5">
              {String(output)}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// 执行日志面板
function ExecutionLogs({ executionId }: { executionId: string }) {
  type LogEntry = { type: 'system' | 'output' | 'stage' | 'error'; text: string };
  type PauseState = { stage_name: string; question: string; options: { label: string; value: string }[] };
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [wsConnected, setWsConnected] = useState(false);
  const [currentStage, setCurrentStage] = useState<string | null>(null);
  const [pauseState, setPauseState] = useState<PauseState | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  // 每次新日志追加时自动滚到底部
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  useEffect(() => {
    setLogs([]);
    setWsConnected(false);
    setCurrentStage(null);
    setPauseState(null);

    const ws = new WebSocket(`/ws/executions/${executionId}`);
    wsRef.current = ws;

    ws.onopen = () => {
      if (wsRef.current !== ws) return;
      setWsConnected(true);
    };

    ws.onmessage = (event) => {
      if (wsRef.current !== ws) return;
      try {
        const data = JSON.parse(event.data);

        if (data.type === 'snapshot') {
          const newLogs: LogEntry[] = [];

          if (data.current_stage) {
            setCurrentStage(data.current_stage);
          }
          // 已完成阶段
          if (data.stage_results?.length > 0) {
            data.stage_results.forEach((sr: { stage_name: string }) => {
              newLogs.push({ type: 'stage', text: `✓ 阶段完成: ${sr.stage_name}` });
            });
          }
          // 历史输出日志（catch-up）
          if (data.output_log?.length > 0) {
            (data.output_log as string[]).forEach((line) => {
              newLogs.push({ type: 'output', text: line });
            });
          }
          if (data.error) {
            newLogs.push({ type: 'error', text: data.error });
          }

          setLogs(newLogs);
        } else if (data.type === 'output') {
          setLogs((prev) => [...prev, { type: 'output', text: data.line }]);
        } else if (data.type === 'stage_started') {
          setCurrentStage(data.stage_name);
          setLogs((prev) => [...prev, { type: 'stage', text: `▶ 阶段开始: ${data.stage_name}` }]);
        } else if (data.type === 'stage_completed') {
          setLogs((prev) => [...prev, { type: 'stage', text: `✓ 阶段完成: ${data.stage_name}` }]);
        } else if (data.type === 'workflow_paused') {
          setLogs((prev) => [...prev, { type: 'system', text: `⏸ 暂停 — ${data.stage_name}` }]);
          setPauseState({ stage_name: data.stage_name, question: data.question, options: data.options ?? [] });
        } else if (data.type === 'workflow_resumed') {
          setPauseState(null);
          setLogs((prev) => [...prev, { type: 'system', text: `▶ 已选择: ${data.chosen_value}` }]);
        } else if (data.type === 'completed') {
          setCurrentStage(null);
          setPauseState(null);
          setLogs((prev) => [...prev, { type: 'system', text: '✓ 工作流执行完成' }]);
          setWsConnected(false);
        } else if (data.type === 'failed') {
          setCurrentStage(null);
          setPauseState(null);
          setLogs((prev) => [...prev, { type: 'error', text: `✗ 执行失败: ${data.error}` }]);
          setWsConnected(false);
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onclose = () => {
      if (wsRef.current !== ws) return;
      setWsConnected(false);
    };

    ws.onerror = () => {
      // onerror always precedes onclose
    };

    return () => {
      wsRef.current = null;
      ws.close();
    };
  }, [executionId]);

  const handleResume = (value: string) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ action: 'resume', execution_id: executionId, value }));
      setPauseState(null);
      setLogs((prev) => [...prev, { type: 'system', text: `▶ 已选择: ${value}` }]);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#1e1e1e] rounded-xl overflow-hidden border border-white/5">
      <div className="flex items-center justify-between px-4 py-3 bg-gradient-to-r from-[#252526] to-[#1e1e1e] border-b border-white/5">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-gray-400" />
          <span className="text-sm text-gray-400">执行日志</span>
          {currentStage && (
            <span className="px-2 py-0.5 text-xs rounded-full bg-blue-500/20 text-blue-400 border border-blue-500/30 animate-pulse">
              {currentStage}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${wsConnected ? 'bg-emerald-500 animate-pulse' : 'bg-gray-600'}`} />
          <span className="text-xs text-gray-500">{wsConnected ? '实时' : '历史'}</span>
        </div>
      </div>

      {/* 暂停等待选项 UI */}
      {pauseState && (
        <div className="px-4 py-4 bg-amber-950/40 border-b border-amber-500/30">
          <div className="flex items-center gap-2 mb-3">
            <PauseCircle className="w-4 h-4 text-amber-400 animate-pulse flex-shrink-0" />
            <p className="text-sm text-amber-300 font-medium">{pauseState.question}</p>
          </div>
          <div className="flex flex-col gap-2">
            {pauseState.options.map((opt) => (
              <button
                key={opt.value}
                onClick={() => handleResume(opt.value)}
                className="w-full text-left px-4 py-2.5 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 hover:border-amber-500/50 text-amber-200 text-sm transition-all"
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex-1 overflow-auto p-4 font-mono text-xs space-y-0.5">
        {logs.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <p className="text-gray-600">等待输出...</p>
          </div>
        ) : (
          logs.map((log, index) => (
            <div
              key={index}
              className={cn(
                'leading-relaxed px-2 py-0.5 rounded whitespace-pre-wrap break-all',
                log.type === 'error'  ? 'text-red-400' :
                log.type === 'stage'  ? 'text-emerald-400 font-semibold' :
                log.type === 'system' ? 'text-blue-400' :
                'text-gray-300'
              )}
            >
              {log.text}
            </div>
          ))
        )}
        <div ref={logsEndRef} />
      </div>
    </div>
  );
}

// 工作流操作说明组件
function WorkflowOperationsGuide() {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <div className="bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5 border border-indigo-500/20 rounded-2xl p-5 relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-r from-indigo-500/5 to-purple-500/5 pointer-events-none" />

      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-indigo-600 hover:text-indigo-700 transition-colors relative"
      >
        <Play className="w-4 h-4" />
        <span>工作流操作说明</span>
        <ChevronRight className={cn('w-4 h-4 transition-transform duration-200', isExpanded && 'rotate-90')} />
      </button>
      {isExpanded && (
        <div className="mt-4 grid grid-cols-2 md:grid-cols-5 gap-3 relative">
          {WORKFLOW_OPERATIONS.map((op) => (
            <div key={op.key} className="flex items-start gap-2 p-2 rounded-lg bg-card/50 border border-border/50">
              <span className="flex-shrink-0 w-5 h-5 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 text-white text-xs flex items-center justify-center font-bold shadow-md">
                {op.key}
              </span>
              <div>
                <span className="font-medium text-sm">{op.action}: </span>
                <span className="text-xs text-muted-foreground">{op.desc}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// 执行卡片组件
function ExecutionCard({
  execution,
  onClick,
  onCancel,
}: {
  execution: Execution;
  onClick: () => void;
  onCancel: (id: string) => void;
}) {
  const config = STATUS_CONFIG[execution.status];
  const Icon = config.icon;

  return (
    <div
      onClick={onClick}
      className={cn(
        'bg-card rounded-2xl border border-border/50 p-5 cursor-pointer',
        'hover:shadow-lg hover:shadow-primary/5 hover:border-primary/20',
        'transition-all duration-200 hover:-translate-y-0.5 group'
      )}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className={cn(
            'p-2.5 rounded-xl bg-gradient-to-br ', config.gradient,
            'shadow-lg group-hover:scale-110 transition-transform duration-200'
          )}>
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <h3 className="font-semibold group-hover:text-indigo-600 transition-colors">{execution.workflow_id}</h3>
            <p className="text-xs text-muted-foreground font-mono">ID: {execution.id.slice(0, 8)}...</p>
          </div>
        </div>
        <span className={cn(
          'px-3 py-1.5 rounded-full text-xs font-medium shadow-md',
          'bg-gradient-to-r ' + config.gradient,
          'text-white'
        )}>
          {config.label}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm">
        <div className="space-y-1">
          <p className="text-muted-foreground text-xs">开始时间</p>
          <p className="font-medium text-sm">{formatTime(execution.started_at)}</p>
        </div>
        <div className="space-y-1">
          <p className="text-muted-foreground text-xs">持续时间</p>
          <p className="font-medium text-sm">{formatDuration(execution.started_at, execution.finished_at)}</p>
        </div>
      </div>

      {(execution.stage_results && execution.stage_results.length > 0) && (
        <div className="mt-4 pt-4 border-t border-border/50">
          <p className="text-xs text-muted-foreground mb-2">
            阶段进度 ({execution.stage_results.length})
          </p>
          <div className="flex items-center gap-2">
            {execution.stage_results.slice(0, 3).map((result, idx) => (
              <span key={idx} className="px-2 py-1 text-xs bg-gradient-to-r from-emerald-500/10 to-green-500/10 text-emerald-600 rounded-lg border border-emerald-500/20">
                {result.stage_name}
              </span>
            ))}
            {execution.stage_results.length > 3 && (
              <span className="text-xs text-muted-foreground">
                +{execution.stage_results.length - 3} 更多
              </span>
            )}
          </div>
        </div>
      )}

      {execution.error && (
        <div className="mt-4 p-3 rounded-xl bg-gradient-to-r from-red-500/10 to-rose-500/10 border border-red-500/20 flex items-start gap-2">
          <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5" />
          <span className="text-sm text-red-600 line-clamp-2">{execution.error}</span>
        </div>
      )}

      {execution.status === 'running' && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onCancel(execution.id);
          }}
          className="absolute top-3 right-3 p-1.5 rounded-lg bg-red-500/10 text-red-500 hover:bg-red-500/20 transition-colors"
        >
          <XCircle className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}

// 主页面组件
export function ExecutionsPage() {
  const location = useLocation();
  const { cancelExecution, connectWebSocket } = useExecutionStore();
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);

  // Use React Query for fetching
  const { executions, loading, refetch } = useExecutionsQuery();

  // Auto-open execution detail when navigated from template launch
  useEffect(() => {
    const state = location.state as { openExecutionId?: string } | null;
    if (state?.openExecutionId) {
      setSelectedExecutionId(state.openExecutionId);
    }
  }, [location.state]);

  // The live execution from store (auto-updates via WS events)
  const liveSelectedExecution = selectedExecutionId
    ? executions.find((e) => e.id === selectedExecutionId) ?? null
    : null;

  // Listen for workspace changes
  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => { unsubscribe(); };
  }, [refetch]);

  // Connect store WS when opening modal for a running execution
  useEffect(() => {
    if (liveSelectedExecution?.status === 'running') {
      connectWebSocket(liveSelectedExecution.id);
    }
  }, [liveSelectedExecution?.id, liveSelectedExecution?.status, connectWebSocket]);

  // Auto-connect WS for ALL running executions so pause events are never missed.
  // Without this, workflow_paused events are lost if the user hasn't manually
  // clicked on the execution to open the detail panel.
  useEffect(() => {
    const running = executions.filter((e) => e.status === 'running');
    running.forEach((e) => connectWebSocket(e.id));
  }, [executions, connectWebSocket]);

  const handleCancel = useCallback(
    (id: string) => {
      cancelExecution(id);
    },
    [cancelExecution]
  );

  if (loading) {
    return (
      <div className="page-container">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="text-center">
            <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center animate-pulse">
              <Loader2 className="w-8 h-8 text-indigo-500 animate-spin" />
            </div>
            <p className="text-muted-foreground">加载中...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="page-container space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              执行记录
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">查看所有工作流执行历史</p>
        </div>
        <span className="px-3 py-1.5 rounded-full bg-indigo-500/10 text-indigo-600 text-sm font-medium border border-indigo-500/20">
          {executions.length} 条记录
        </span>
      </div>

      <WorkflowOperationsGuide />

      {executions.length === 0 ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <Clock className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无执行记录</h3>
          <p className="text-muted-foreground mb-4">从工作流列表选择一个工作流开始执行</p>
          <button className="btn-primary">
            <Play className="w-4 h-4" />
            前往工作流
          </button>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-2 stagger-children">
          {executions.map((execution) => (
            <ExecutionCard
              key={execution.id}
              execution={execution}
              onClick={() => setSelectedExecutionId(execution.id)}
              onCancel={handleCancel}
            />
          ))}
        </div>
      )}

      {liveSelectedExecution && (
        <ExecutionDetailModal
          execution={liveSelectedExecution}
          onClose={() => setSelectedExecutionId(null)}
          onCancel={handleCancel}
        />
      )}

    </div>
  );
}
