import { useCallback, useEffect, useRef, useState } from 'react';
import { WS_BASE_URL } from '@/api/constants';

export type ServiceStatus = 'idle' | 'starting' | 'running' | 'stopping' | 'error';

export interface ServiceOutputLine {
  type: 'stdout' | 'stderr' | 'system';
  data: string;
}

export interface UseServiceRunnerReturn {
  status: ServiceStatus;
  pid: number | null;
  lastLine: string;
  output: ServiceOutputLine[];
  exitCode: number | null;
  error: string | null;
  start: (command: string, cwd: string) => void;
  stop: () => void;
  clearOutput: () => void;
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
  const [output, setOutput] = useState<ServiceOutputLine[]>([]);
  const [exitCode, setExitCode] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const outputRef = useRef<ServiceOutputLine[]>([]);

  const appendOutput = useCallback((line: ServiceOutputLine) => {
    outputRef.current = [...outputRef.current, line];
    setOutput([...outputRef.current]);
  }, []);

  const clearOutput = useCallback(() => {
    outputRef.current = [];
    setOutput([]);
    setExitCode(null);
    setError(null);
    setLastLine('');
  }, []);

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

  const start = useCallback(
    (command: string, cwd: string) => {
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

      // Reset state for new run
      outputRef.current = [];
      setOutput([]);
      setExitCode(null);
      setStatus('starting');
      setError(null);
      setPid(null);
      setLastLine('正在连接...');
      appendOutput({ type: 'system', data: `$ ${command}` });

      const ws = new WebSocket(`${WS_BASE_URL}/ws/run-command`);
      wsRef.current = ws;

      ws.onopen = () => {
        setLastLine(`$ ${command}`);
        ws.send(
          JSON.stringify({
            type: 'execute',
            command,
            working_directory: cwd,
          }),
        );
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
              if (msg.data) {
                setLastLine(msg.data.trimEnd());
                appendOutput({ type: 'stdout', data: msg.data });
              }
              break;
            case 'stderr':
              if (msg.data) {
                setLastLine(msg.data.trimEnd());
                appendOutput({ type: 'stderr', data: msg.data });
              }
              break;
            case 'exit':
              setStatus('idle');
              setPid(null);
              setExitCode(msg.code ?? -1);
              setLastLine(`进程退出 (code: ${msg.code ?? -1})`);
              appendOutput({
                type: 'system',
                data: `进程退出，代码: ${msg.code ?? -1}`,
              });
              wsRef.current = null;
              break;
            case 'error':
              setStatus('error');
              setError(msg.message ?? '未知错误');
              setPid(null);
              appendOutput({ type: 'stderr', data: msg.message ?? '未知错误' });
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
        appendOutput({ type: 'stderr', data: 'WebSocket 连接失败' });
        wsRef.current = null;
      };

      ws.onclose = () => {
        if (status === 'running' || status === 'starting') {
          setStatus('idle');
          setPid(null);
        }
        wsRef.current = null;
      };
    },
    [status, appendOutput],
  );

  return { status, pid, lastLine, output, exitCode, error, start, stop, clearOutput };
}
