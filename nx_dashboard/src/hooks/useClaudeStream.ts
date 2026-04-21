import { useCallback, useEffect, useRef, useState } from 'react';
import { WS_BASE_URL } from '@/api/constants';

// Claude Stream WebSocket 消息类型
export interface ClaudeStreamStarted {
  type: 'started';
  execution_id: string;
}

export interface ClaudeStreamOutput {
  type: 'output';
  execution_id: string;
  line: string;
}

export interface ClaudeStreamError {
  type: 'error';
  execution_id: string;
  line: string;
}

export interface ClaudeStreamCompleted {
  type: 'completed';
  execution_id: string;
  exit_code: number;
}

export interface ClaudeStreamFailed {
  type: 'failed';
  execution_id: string;
  error: string;
}

export type ClaudeStreamMessage =
  | ClaudeStreamStarted
  | ClaudeStreamOutput
  | ClaudeStreamError
  | ClaudeStreamCompleted
  | ClaudeStreamFailed;

export interface UseClaudeStreamOptions {
  onOutput?: (line: string, isError: boolean) => void;
  onComplete?: (exitCode: number) => void;
  onError?: (error: string) => void;
}

export interface UseClaudeStreamReturn {
  isConnected: boolean;
  isExecuting: boolean;
  executionId: string | null;
  output: string[];
  error: string | null;
  execute: (prompt: string, workingDirectory?: string) => void;
  cancel: () => void;
  clear: () => void;
}

/**
 * Claude CLI 流式执行 Hook
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { isConnected, output, execute, cancel } = useClaudeStream({
 *     onOutput: (line) => console.log(line),
 *     onComplete: (exitCode) => console.log('Done:', exitCode),
 *   });
 *
 *   return (
 *     <div>
 *       <button onClick={() => execute("Say hello")}>执行</button>
 *       <button onClick={cancel}>取消</button>
 *       <pre>{output.join('\n')}</pre>
 *     </div>
 *   );
 * }
 * ```
 */
export function useClaudeStream(options: UseClaudeStreamOptions = {}): UseClaudeStreamReturn {
  const { onOutput, onComplete, onError } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [executionId, setExecutionId] = useState<string | null>(null);
  const [output, setOutput] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const outputRef = useRef<string[]>([]);

  // 清除输出
  const clear = useCallback(() => {
    setOutput([]);
    setError(null);
    outputRef.current = [];
  }, []);

  // 取消执行
  const cancel = useCallback(() => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'cancel' }));
      setIsExecuting(false);
    }
  }, []);

  // 执行 Claude CLI
  const execute = useCallback((prompt: string, workingDirectory?: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      setError('WebSocket 未连接');
      return;
    }

    // 清除之前的输出
    clear();
    setIsExecuting(true);

    wsRef.current.send(JSON.stringify({
      type: 'execute',
      prompt,
      working_directory: workingDirectory || null,
    }));
  }, [clear]);

  // 连接 WebSocket
  useEffect(() => {
    const wsUrl = `${WS_BASE_URL}/ws/claude-stream`;

    console.log('[ClaudeStream] 尝试连接 WebSocket:', wsUrl);

    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('[ClaudeStream] WebSocket 已连接');
      setIsConnected(true);
      setError(null);
    };

    ws.onclose = (event) => {
      console.log('[ClaudeStream] WebSocket 关闭:', event.code, event.reason);
      setIsConnected(false);
      setIsExecuting(false);
    };

    ws.onerror = (event) => {
      console.error('[ClaudeStream] WebSocket 错误:', event);
      setError('WebSocket 连接错误');
      setIsConnected(false);
    };

    ws.onmessage = (event) => {
      console.log('[ClaudeStream] 收到消息:', event.data);
      try {
        const msg: ClaudeStreamMessage = JSON.parse(event.data);

        switch (msg.type) {
          case 'started':
            setExecutionId(msg.execution_id);
            break;

          case 'output':
            outputRef.current = [...outputRef.current, msg.line];
            setOutput([...outputRef.current]);
            onOutput?.(msg.line, false);
            break;

          case 'error':
            outputRef.current = [...outputRef.current, `[stderr] ${msg.line}`];
            setOutput([...outputRef.current]);
            onOutput?.(msg.line, true);
            break;

          case 'completed':
            setIsExecuting(false);
            onComplete?.(msg.exit_code);
            break;

          case 'failed':
            setIsExecuting(false);
            setError(msg.error);
            onError?.(msg.error);
            break;
        }
      } catch {
        // 忽略解析错误
      }
    };

    return () => {
      ws.close();
    };
  }, [onOutput, onComplete, onError]);

  return {
    isConnected,
    isExecuting,
    executionId,
    output,
    error,
    execute,
    cancel,
    clear,
  };
}
