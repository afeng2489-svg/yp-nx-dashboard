import { useState, useEffect, useRef, useCallback } from 'react';
import { useExecutionStore, Execution, StageResult } from '@/stores/executionStore';
import { Agent } from '@/stores/workflowStore';
import { A2UIPanel } from '@/components/a2ui';
import {
  Square,
  Clock,
  CheckCircle,
  XCircle,
  AlertCircle,
  Loader2,
  ChevronDown,
  ChevronRight,
  Terminal,
  Wifi,
  WifiOff,
  MessageSquare,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { WS_BASE_URL } from '@/api/constants';
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select';

// WebSocket 日志流 Hook
function useLogStream(executionId: string | undefined) {
  const [logs, setLogs] = useState<string[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (!executionId) return;

    const wsUrl = `${WS_BASE_URL}/ws/executions/${executionId}`;

    try {
      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        setIsConnected(true);
        setError(null);
        setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] 日志流已连接`]);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          // Handle ExecutionEvent types from backend
          switch (msg.type) {
            case 'started':
              setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] 执行已启动`]);
              break;
            case 'status_changed':
              setLogs((prev) => [
                ...prev,
                `[${new Date().toLocaleTimeString()}] 状态变更: ${msg.status}`,
              ]);
              break;
            case 'stage_started':
              setLogs((prev) => [
                ...prev,
                `[${new Date().toLocaleTimeString()}] 阶段开始: ${msg.stage_name}`,
              ]);
              break;
            case 'stage_completed':
              setLogs((prev) => [
                ...prev,
                `[${new Date().toLocaleTimeString()}] 阶段完成: ${msg.stage_name}`,
              ]);
              break;
            case 'output':
              setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg.line}`]);
              break;
            case 'completed':
              setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] 执行完成`]);
              break;
            case 'failed':
              setLogs((prev) => [
                ...prev,
                `[${new Date().toLocaleTimeString()}] 执行失败: ${msg.error}`,
              ]);
              break;
            default:
              setLogs((prev) => [
                ...prev,
                `[${new Date().toLocaleTimeString()}] ${JSON.stringify(msg)}`,
              ]);
          }
        } catch {
          setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${event.data}`]);
        }
      };

      ws.onclose = () => {
        setIsConnected(false);
        setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] 日志流已断开`]);
      };

      ws.onerror = () => {
        setError('连接失败');
        setIsConnected(false);
      };

      wsRef.current = ws;
    } catch {
      setError('连接初始化失败');
    }
  }, [executionId]);

  useEffect(() => {
    if (executionId) {
      connect();
    }
    const reconnectTimeout = reconnectTimeoutRef.current;
    const ws = wsRef.current;
    return () => {
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }
      if (ws) {
        ws.close();
      }
    };
  }, [executionId, connect]);

  return { logs, isConnected, error };
}

// 状态图标映射
const STATUS_ICONS = {
  pending: <Clock className="w-4 h-4 text-gray-400" />,
  running: <Loader2 className="w-4 h-4 text-yellow-500 animate-spin" />,
  paused: <Clock className="w-4 h-4 text-amber-500" />,
  completed: <CheckCircle className="w-4 h-4 text-green-500" />,
  failed: <XCircle className="w-4 h-4 text-red-500" />,
  cancelled: <Square className="w-4 h-4 text-gray-400" />,
  interrupted: <AlertCircle className="w-4 h-4 text-orange-400" />,
} as const;

// 代理状态
interface AgentStatus {
  agent: Agent;
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress: number;
  output?: string;
}

// 阶段进度卡片
function StageCard({
  stageName,
  stageResult,
  agents,
}: {
  stageName: string;
  stageResult?: StageResult;
  agents: AgentStatus[];
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  const completedAgents = agents.filter((a) => a.status === 'completed').length;
  const totalAgents = agents.length;
  const progress = totalAgents > 0 ? (completedAgents / totalAgents) * 100 : 0;

  return (
    <div className="border rounded-lg overflow-hidden">
      {/* 阶段头部 */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-4 py-3 bg-accent hover:bg-accent/80 transition-colors"
      >
        <div className="flex items-center gap-3">
          {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          <span className="font-medium">{stageName}</span>
          <span className="text-sm text-muted-foreground">
            {completedAgents}/{totalAgents} 代理
          </span>
        </div>
        <div className="flex items-center gap-3">
          {/* 进度条 */}
          <div className="w-24 h-2 bg-secondary rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
          {stageResult?.completed_at && (
            <span className="text-xs text-muted-foreground">
              完成于 {new Date(stageResult.completed_at).toLocaleTimeString()}
            </span>
          )}
        </div>
      </button>

      {/* 代理列表 */}
      {isExpanded && (
        <div className="divide-y">
          {agents.map((agentStatus) => (
            <AgentRow key={agentStatus.agent.id} agentStatus={agentStatus} />
          ))}
        </div>
      )}
    </div>
  );
}

// 代理状态行
function AgentRow({ agentStatus }: { agentStatus: AgentStatus }) {
  const { agent, status, progress, output } = agentStatus;

  return (
    <div className="px-4 py-3 hover:bg-accent/50 transition-colors">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {STATUS_ICONS[status]}
          <div>
            <p className="font-medium text-sm">{agent.role}</p>
            <p className="text-xs text-muted-foreground">{agent.model}</p>
          </div>
        </div>
        {status === 'running' && (
          <div className="flex items-center gap-2">
            <div className="w-20 h-1.5 bg-secondary rounded-full overflow-hidden">
              <div className="h-full bg-primary animate-pulse" style={{ width: `${progress}%` }} />
            </div>
            <span className="text-xs text-muted-foreground">{progress}%</span>
          </div>
        )}
      </div>
      {output && (
        <div className="mt-2 p-2 bg-[#1e1e1e] rounded text-xs text-gray-300 font-mono overflow-auto max-h-20">
          {output}
        </div>
      )}
    </div>
  );
}

// 日志流面板
function LogStreamPanel({ executionId }: { executionId: string }) {
  const { logs, isConnected, error } = useLogStream(executionId);
  const logsEndRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  return (
    <div className="flex flex-col h-full bg-[#1e1e1e] rounded-lg overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-[#252526] border-b border-[#3c3c3c]">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-gray-400" />
          <span className="text-sm text-gray-400">执行日志</span>
        </div>
        <div className="flex items-center gap-2">
          {isConnected ? (
            <Wifi className="w-3.5 h-3.5 text-green-500" />
          ) : error ? (
            <WifiOff className="w-3.5 h-3.5 text-red-500" />
          ) : (
            <Loader2 className="w-3.5 h-3.5 text-yellow-500 animate-spin" />
          )}
          <span
            className={cn(
              'text-xs',
              isConnected ? 'text-green-500' : error ? 'text-red-500' : 'text-yellow-500',
            )}
          >
            {isConnected ? '已连接' : error ? '连接失败' : '连接中...'}
          </span>
        </div>
      </div>
      <div className="flex-1 overflow-auto p-3 font-mono text-xs">
        {logs.length === 0 ? (
          <div className="text-center text-gray-500 py-4">等待日志数据...</div>
        ) : (
          logs.map((log, index) => (
            <div key={index} className="text-gray-300 leading-relaxed whitespace-pre-wrap">
              {log}
            </div>
          ))
        )}
        <div ref={logsEndRef} />
      </div>
    </div>
  );
}

// 执行详情面板
function ExecutionDetail({ execution }: { execution: Execution }) {
  const [activeTab, setActiveTab] = useState<'stages' | 'logs'>('stages');

  // 模拟每个阶段的代理状态
  const stageAgents: Record<string, AgentStatus[]> = {};
  execution.stage_results?.forEach((result) => {
    stageAgents[result.stage_name] = (result.outputs || []).map((output, idx) => ({
      agent: {
        id: `${result.stage_name}-agent-${idx}`,
        role: `Agent ${idx + 1}`,
        model: 'claude-sonnet-4',
        prompt: '',
        depends_on: [],
      },
      status: 'completed' as const,
      progress: 100,
      output: String(output),
    }));
  });

  return (
    <div className="flex flex-col h-full">
      {/* 执行信息头部 */}
      <div className="px-4 py-3 border-b bg-card">
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold">执行详情</h3>
          <div className="flex items-center gap-2">
            {STATUS_ICONS[execution.status]}
            <span className="text-sm font-medium">{execution.status}</span>
          </div>
        </div>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-muted-foreground">工作流: </span>
            <span>{execution.workflow_id}</span>
          </div>
          <div>
            <span className="text-muted-foreground">开始时间: </span>
            <span>
              {execution.started_at ? new Date(execution.started_at).toLocaleString() : '-'}
            </span>
          </div>
        </div>
      </div>

      {/* Tab 切换 */}
      <div className="flex border-b">
        <button
          onClick={() => setActiveTab('stages')}
          className={cn(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            activeTab === 'stages'
              ? 'border-b-2 border-primary text-primary'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          阶段进度
        </button>
        <button
          onClick={() => setActiveTab('logs')}
          className={cn(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            activeTab === 'logs'
              ? 'border-b-2 border-primary text-primary'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          实时日志
        </button>
      </div>

      {/* 内容区域 */}
      <div className="flex-1 overflow-auto p-4">
        {activeTab === 'stages' ? (
          <div className="space-y-3">
            {execution.stage_results?.map((result) => (
              <StageCard
                key={result.stage_name}
                stageName={result.stage_name}
                stageResult={result}
                agents={
                  stageAgents[result.stage_name] || [
                    {
                      agent: {
                        id: 'default',
                        role: '默认代理',
                        model: 'claude-sonnet-4',
                        prompt: '',
                        depends_on: [],
                      },
                      status: 'pending' as const,
                      progress: 0,
                    },
                  ]
                }
              />
            ))}
            {(!execution.stage_results || execution.stage_results.length === 0) && (
              <div className="text-center py-8 text-muted-foreground">暂无阶段数据</div>
            )}
          </div>
        ) : (
          <LogStreamPanel executionId={execution.id} />
        )}
      </div>
    </div>
  );
}

// 代理列表视图
function AgentListView({ execution }: { execution: Execution }) {
  const [agents, setAgents] = useState<AgentStatus[]>([]);

  // 模拟代理状态
  useEffect(() => {
    const mockAgents: AgentStatus[] = [
      {
        agent: {
          id: 'planner',
          role: '规划器',
          model: 'claude-sonnet-4',
          prompt: '负责任务分解和规划',
          depends_on: [],
        },
        status: execution.status === 'completed' ? 'completed' : 'running',
        progress: execution.status === 'completed' ? 100 : 75,
        output: '已完成任务分解，生成 5 个子任务',
      },
      {
        agent: {
          id: 'executor',
          role: '执行器',
          model: 'claude-sonnet-4',
          prompt: '负责执行具体任务',
          depends_on: ['planner'],
        },
        status: execution.status === 'completed' ? 'completed' : 'running',
        progress: execution.status === 'completed' ? 100 : 50,
        output: '正在执行代码生成...',
      },
      {
        agent: {
          id: 'reviewer',
          role: '审查器',
          model: 'claude-sonnet-4',
          prompt: '负责代码审查和质量检查',
          depends_on: ['executor'],
        },
        status: 'pending',
        progress: 0,
      },
    ];

    setAgents(mockAgents);
  }, [execution]);

  return (
    <div className="space-y-3">
      {agents.map((agentStatus) => (
        <div
          key={agentStatus.agent.id}
          className="border rounded-lg p-4 hover:bg-accent/50 transition-colors"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3">
              {STATUS_ICONS[agentStatus.status]}
              <div>
                <p className="font-medium">{agentStatus.agent.role}</p>
                <p className="text-sm text-muted-foreground">{agentStatus.agent.model}</p>
                {agentStatus.output && (
                  <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
                    {agentStatus.output}
                  </p>
                )}
              </div>
            </div>
            {agentStatus.status === 'running' && (
              <div className="flex items-center gap-2">
                <div className="w-24 h-1.5 bg-secondary rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all"
                    style={{ width: `${agentStatus.progress}%` }}
                  />
                </div>
                <span className="text-xs text-muted-foreground">{agentStatus.progress}%</span>
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

// 主执行监控组件
export function ExecutionMonitor() {
  const { executions, currentExecution, fetchExecutions, setCurrentExecution } =
    useExecutionStore();
  const [viewMode, setViewMode] = useState<'detail' | 'agents' | 'interaction'>('detail');

  useEffect(() => {
    fetchExecutions();
  }, [fetchExecutions]);

  // 默认显示最新的执行
  const displayExecution = currentExecution || executions[0];

  if (!displayExecution) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <AlertCircle className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>暂无执行记录</p>
          <p className="text-sm mt-1">选择一个工作流开始执行</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-card border rounded-lg overflow-hidden">
      {/* 执行选择器 */}
      <div className="px-4 py-3 border-b bg-accent/30">
        <Select
          value={displayExecution.id}
          onValueChange={(v) => {
            const exec = executions.find((ex) => ex.id === v);
            if (exec) setCurrentExecution(exec);
          }}
        >
          <SelectTrigger className="h-8 text-sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            {executions.map((exec) => (
              <SelectItem key={exec.id} value={exec.id}>
                {exec.workflow_id} - {exec.status} (
                {exec.started_at ? new Date(exec.started_at).toLocaleString() : '未开始'})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* 视图切换 */}
      <div className="flex border-b">
        <button
          onClick={() => setViewMode('detail')}
          className={cn(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            viewMode === 'detail'
              ? 'border-b-2 border-primary text-primary'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          详细信息
        </button>
        <button
          onClick={() => setViewMode('agents')}
          className={cn(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            viewMode === 'agents'
              ? 'border-b-2 border-primary text-primary'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          代理视图
        </button>
        <button
          onClick={() => setViewMode('interaction')}
          className={cn(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors flex items-center justify-center gap-2',
            viewMode === 'interaction'
              ? 'border-b-2 border-primary text-primary'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          <MessageSquare className="w-4 h-4" />
          交互
        </button>
      </div>

      {/* 内容 */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'detail' ? (
          <ExecutionDetail key={displayExecution.id} execution={displayExecution} />
        ) : viewMode === 'agents' ? (
          <div className="p-4">
            <AgentListView key={displayExecution.id} execution={displayExecution} />
          </div>
        ) : (
          <A2UIPanel key={displayExecution.id} executionId={displayExecution.id} />
        )}
      </div>
    </div>
  );
}
