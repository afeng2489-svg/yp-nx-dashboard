import { useState, useEffect } from 'react';
import { Activity, Ban, RefreshCw, Clock, CheckCircle, XCircle, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { API_BASE_URL } from '@/api/constants';

interface RunningProcess {
  execution_id: string;
  pid: number | null;
  role_id: string;
  role_name: string;
  team_id: string;
  task: string;
  start_time: string;
  elapsed_secs: number;
  status: string;
  output: string;
}

export function ProcessMonitor() {
  const [processes, setProcesses] = useState<RunningProcess[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch processes
  const fetchProcesses = async () => {
    try {
      setLoading(true);
      const response = await fetch(`${API_BASE_URL}/api/v1/processes`);
      if (!response.ok) throw new Error('Failed to fetch processes');
      const data = await response.json();
      setProcesses(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  // Kill a process
  const killProcess = async (executionId: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/processes/${executionId}/kill`, {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to kill process');
      await fetchProcesses();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to kill process');
    }
  };

  // Auto-refresh every 5 seconds
  useEffect(() => {
    fetchProcesses();
    const interval = setInterval(fetchProcesses, 5000);
    return () => clearInterval(interval);
  }, []);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'running':
        return <Loader2 className="w-4 h-4 animate-spin text-blue-500" />;
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'failed':
      case 'killed':
        return <XCircle className="w-4 h-4 text-red-500" />;
      default:
        return <Activity className="w-4 h-4 text-gray-500" />;
    }
  };

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50 bg-gradient-to-r from-red-500/5 to-orange-500/5">
        <div className="flex items-center gap-2">
          <Activity className="w-5 h-5 text-red-500" />
          <h2 className="font-semibold">进程监测</h2>
          <span className={cn(
            'px-2 py-0.5 text-xs rounded-full',
            processes.length > 0 ? 'bg-red-500/20 text-red-500' : 'bg-gray-500/20 text-gray-500'
          )}>
            {processes.length} 个进程
          </span>
        </div>
        <button
          onClick={fetchProcesses}
          disabled={loading}
          className="p-2 rounded-lg hover:bg-accent transition-colors"
          title="刷新"
        >
          <RefreshCw className={cn('w-4 h-4', loading && 'animate-spin')} />
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-4 mt-3 px-3 py-2 bg-red-500/10 border border-red-500/20 rounded-lg text-sm text-red-500">
          {error}
        </div>
      )}

      {/* Process List */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {processes.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
            <Activity className="w-12 h-12 mb-4 opacity-50" />
            <p className="font-medium">暂无运行中的进程</p>
            <p className="text-sm mt-1">正在执行的 Claude 任务会显示在这里</p>
          </div>
        ) : (
          processes.map((process) => (
            <div
              key={process.execution_id}
              className="bg-card border border-border/50 rounded-lg overflow-hidden"
            >
              {/* Process Header */}
              <div className="flex items-center justify-between px-3 py-2 bg-muted/30">
                <div className="flex items-center gap-2">
                  {getStatusIcon(process.status)}
                  <span className="font-medium text-sm">{process.role_name}</span>
                  {process.pid && (
                    <span className="text-xs text-muted-foreground">PID: {process.pid}</span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground flex items-center gap-1">
                    <Clock className="w-3 h-3" />
                    {formatDuration(process.elapsed_secs)}
                  </span>
                  {process.status === 'running' && (
                    <button
                      onClick={() => killProcess(process.execution_id)}
                      className="p-1 rounded hover:bg-red-500/20 text-red-500 transition-colors"
                      title="终止进程"
                    >
                      <Ban className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              {/* Task */}
              <div className="px-3 py-2 border-t border-border/30">
                <p className="text-xs text-muted-foreground mb-1">任务:</p>
                <p className="text-sm line-clamp-2">{process.task}</p>
              </div>

              {/* Output */}
              {process.output && (
                <div className="px-3 py-2 border-t border-border/30 bg-black/20">
                  <p className="text-xs text-muted-foreground mb-1">输出:</p>
                  <pre className="text-xs font-mono text-gray-300 whitespace-pre-wrap max-h-32 overflow-y-auto">
                    {process.output}
                  </pre>
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
