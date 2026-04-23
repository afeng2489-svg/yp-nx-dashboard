import { useCallback, useEffect, useRef, useState } from 'react';
import { WS_BASE_URL } from '@/api/constants';

export type ServiceStatus = 'idle' | 'starting' | 'running' | 'stopping' | 'error';

export interface UseServiceRunnerReturn {
  status: ServiceStatus;
  pid: number | null;
  lastLine: string;
  error: string | null;
  start: (command: string, cwd: string) => void;
  stop: () => void;
}

interface ServerMsg {
  type: 'started' | 'stdout' | 'stderr' | 'exit' | 'error';
  pid?: number;
  data?: string;
  code?: number;
  message?: string;
}

export function useServiceRunner(): UseServiceRunnerReturn {
  const [status, setStatus] = useState<ServiceStatus>('idle');
  const [pid, setPid] = useState<number | null>(null);
  const [lastLine, setLastLine] = useState('');
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);

  const stop = useCallback(() => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      setStatus('stopping');
      wsRef.current.send(JSON.stringify({ type: 'cancel' }));
    }
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      wsRef.current?.close();
    };
  }, []);

  const start = useCallback((command: string, cwd: string) => {
    if (!command.trim() || !cwd.trim()) {
      setError('命令或工作目录未配置');
      setStatus('error');
      return;
    }

    // Close any existing connection
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setStatus('starting');
    setError(null);
    setPid(null);
    setLastLine('正在连接...');

    const ws = new WebSocket(`${WS_BASE_URL}/ws/run-command`);
    wsRef.current = ws;

    ws.onopen = () => {
      setLastLine(`$ ${command}`);
      ws.send(JSON.stringify({
        type: 'execute',
        command,
        working_directory: cwd,
      }));
    };

    ws.onmessage = (event) => {
      try {
        const msg: ServerMsg = JSON.parse(event.data);
        switch (msg.type) {
          case 'started':
            setPid(msg.pid ?? null);
            setStatus('running');
            break;
          case 'stdout':
          case 'stderr':
            if (msg.data) setLastLine(msg.data.trimEnd());
            break;
          case 'exit':
            setStatus('idle');
            setPid(null);
            setLastLine(`进程退出 (code: ${msg.code ?? -1})`);
            wsRef.current = null;
            break;
          case 'error':
            setStatus('error');
            setError(msg.message ?? '未知错误');
            setPid(null);
            wsRef.current = null;
            break;
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onerror = () => {
      setStatus('error');
      setError('WebSocket 连接失败');
      wsRef.current = null;
    };

    ws.onclose = () => {
      if (status === 'running' || status === 'starting') {
        setStatus('idle');
        setPid(null);
      }
      wsRef.current = null;
    };
  }, [status]);

  return { status, pid, lastLine, error, start, stop };
}
