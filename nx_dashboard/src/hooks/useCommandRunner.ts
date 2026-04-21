import { useCallback, useEffect, useRef, useState } from 'react';
import { WS_BASE_URL } from '@/api/constants';

export interface UseCommandRunnerReturn {
  isConnected: boolean;
  isRunning: boolean;
  pid: number | null;
  output: OutputLine[];
  exitCode: number | null;
  error: string | null;
  execute: (command: string, workingDirectory: string) => void;
  cancel: () => void;
  clear: () => void;
}

export interface OutputLine {
  type: 'stdout' | 'stderr' | 'system';
  data: string;
  timestamp: number;
}

interface RunCommandServerMsg {
  type: 'started' | 'stdout' | 'stderr' | 'exit' | 'error';
  pid?: number;
  data?: string;
  code?: number;
  message?: string;
}

export function useCommandRunner(): UseCommandRunnerReturn {
  const [isConnected, setIsConnected] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [pid, setPid] = useState<number | null>(null);
  const [output, setOutput] = useState<OutputLine[]>([]);
  const [exitCode, setExitCode] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const outputRef = useRef<OutputLine[]>([]);
  const reconnectRef = useRef(0);

  const appendOutput = useCallback((line: OutputLine) => {
    outputRef.current = [...outputRef.current, line];
    setOutput([...outputRef.current]);
  }, []);

  const clear = useCallback(() => {
    outputRef.current = [];
    setOutput([]);
    setExitCode(null);
    setError(null);
    setPid(null);
  }, []);

  const cancel = useCallback(() => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'cancel' }));
      setIsRunning(false);
    }
  }, []);

  const execute = useCallback((command: string, workingDirectory: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      setError('WebSocket 未连接');
      return;
    }

    clear();
    setIsRunning(true);
    setExitCode(null);
    appendOutput({ type: 'system', data: `$ ${command}`, timestamp: Date.now() });

    wsRef.current.send(JSON.stringify({
      type: 'execute',
      command,
      working_directory: workingDirectory,
    }));
  }, [clear, appendOutput]);

  // Connect WebSocket
  useEffect(() => {
    const connect = () => {
      const wsUrl = `${WS_BASE_URL}/ws/run-command`;
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        setError(null);
        reconnectRef.current = 0;
      };

      ws.onclose = () => {
        setIsConnected(false);
        setIsRunning(false);
        wsRef.current = null;

        // Reconnect with backoff
        if (reconnectRef.current < 5) {
          const delay = Math.min(1000 * Math.pow(2, reconnectRef.current), 10000);
          reconnectRef.current += 1;
          setTimeout(connect, delay);
        }
      };

      ws.onerror = () => {
        setError('WebSocket 连接错误');
      };

      ws.onmessage = (event) => {
        try {
          const msg: RunCommandServerMsg = JSON.parse(event.data);

          switch (msg.type) {
            case 'started':
              setPid(msg.pid ?? null);
              break;
            case 'stdout':
              appendOutput({ type: 'stdout', data: msg.data ?? '', timestamp: Date.now() });
              break;
            case 'stderr':
              appendOutput({ type: 'stderr', data: msg.data ?? '', timestamp: Date.now() });
              break;
            case 'exit':
              setIsRunning(false);
              setExitCode(msg.code ?? -1);
              appendOutput({
                type: 'system',
                data: `进程退出，代码: ${msg.code ?? -1}`,
                timestamp: Date.now(),
              });
              break;
            case 'error':
              setIsRunning(false);
              setError(msg.message ?? '未知错误');
              appendOutput({
                type: 'stderr',
                data: msg.message ?? '未知错误',
                timestamp: Date.now(),
              });
              break;
          }
        } catch {
          // ignore parse errors
        }
      };
    };

    connect();

    return () => {
      reconnectRef.current = 999; // prevent reconnect on unmount
      wsRef.current?.close();
    };
  }, [appendOutput]);

  return {
    isConnected,
    isRunning,
    pid,
    output,
    exitCode,
    error,
    execute,
    cancel,
    clear,
  };
}
