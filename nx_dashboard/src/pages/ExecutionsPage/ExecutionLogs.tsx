import { useEffect, useRef, useState } from 'react';
import { Terminal, PauseCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { api, type ExecutionLog } from '@/api/client';
import { useExecutionWebSocket } from './useExecutionWebSocket';

export interface ExecutionLogsProps {
  executionId: string;
}

export function ExecutionLogs({ executionId }: ExecutionLogsProps) {
  const { logs, wsConnected, currentStage, pauseState, handleResume } =
    useExecutionWebSocket(executionId);
  const [logTab, setLogTab] = useState<'realtime' | 'trace'>('realtime');
  const [traceLogs, setTraceLogs] = useState<ExecutionLog[]>([]);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (logTab === 'trace') {
      api
        .listExecutionLogs(executionId)
        .then(setTraceLogs)
        .catch(() => {});
    }
  }, [logTab, executionId]);

  // 每次新日志追加时自动滚到底部
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  return (
    <div className="flex flex-col h-full bg-[#1e1e1e] rounded-xl overflow-hidden border border-white/5">
      {/* 头部工具栏 */}
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
          <button
            onClick={() => setLogTab('realtime')}
            className={`text-xs px-2 py-0.5 rounded ${logTab === 'realtime' ? 'bg-white/10 text-white' : 'text-gray-500 hover:text-gray-300'}`}
          >
            实时
          </button>
          <button
            onClick={() => setLogTab('trace')}
            className={`text-xs px-2 py-0.5 rounded ${logTab === 'trace' ? 'bg-white/10 text-white' : 'text-gray-500 hover:text-gray-300'}`}
          >
            追踪
          </button>
          <div
            className={`w-2 h-2 rounded-full ${wsConnected ? 'bg-emerald-500 animate-pulse' : 'bg-gray-600'}`}
          />
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

      {/* 日志内容 */}
      <div className="flex-1 overflow-auto p-4 font-mono text-xs space-y-0.5">
        {logTab === 'trace' ? (
          <TraceLogList traceLogs={traceLogs} />
        ) : (
          <>
            <RealtimeLogList logs={logs} />
            <div ref={logsEndRef} />
          </>
        )}
      </div>
    </div>
  );
}

function TraceLogList({ traceLogs }: { traceLogs: ExecutionLog[] }) {
  if (traceLogs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-gray-600">暂无追踪日志</p>
      </div>
    );
  }

  return (
    <>
      {traceLogs.map((log) => (
        <div
          key={log.id}
          className={cn(
            'leading-relaxed px-2 py-1 rounded border-b border-white/5',
            log.status === 'failed'
              ? 'text-red-400'
              : log.status === 'escalated'
                ? 'text-amber-400'
                : 'text-gray-300',
          )}
        >
          <span className="text-gray-500">{log.timestamp.slice(11, 19)}</span>{' '}
          <span
            className={cn(
              'font-semibold',
              log.status === 'failed'
                ? 'text-red-400'
                : log.status === 'escalated'
                  ? 'text-amber-400'
                  : 'text-emerald-400',
            )}
          >
            [{log.status}]
          </span>{' '}
          {log.stage_name ?? '-'}
          {log.model && <span className="text-blue-400 ml-1">@{log.model}</span>}
          {log.attempt > 0 && <span className="text-gray-500 ml-1">attempt={log.attempt}</span>}
          {log.error && <span className="text-red-400 ml-1">— {log.error.slice(0, 80)}</span>}
        </div>
      ))}
    </>
  );
}

function RealtimeLogList({ logs }: { logs: { type: string; text: string }[] }) {
  if (logs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-gray-600">等待输出...</p>
      </div>
    );
  }

  return (
    <>
      {logs.map((log, index) => (
        <div
          key={index}
          className={cn(
            'leading-relaxed px-2 py-0.5 rounded whitespace-pre-wrap break-all',
            log.type === 'error'
              ? 'text-red-400'
              : log.type === 'stage'
                ? 'text-emerald-400 font-semibold'
                : log.type === 'system'
                  ? 'text-blue-400'
                  : 'text-gray-300',
          )}
        >
          {log.text}
        </div>
      ))}
    </>
  );
}
