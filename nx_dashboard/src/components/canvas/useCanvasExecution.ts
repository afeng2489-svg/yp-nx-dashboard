import { useEffect, useRef } from 'react';
import { useCanvasStore } from '@/stores/canvasStore';

interface WsEvent {
  type: string;
  stage_name?: string;
  status?: string;
  duration_ms?: number;
  error?: string;
  tokens?: number;
}

export function useCanvasExecution(executionId: string | null) {
  const { updateNodeExecStatus } = useCanvasStore();
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!executionId) return;
    const ws = new WebSocket(`ws://localhost:3000/ws/executions/${executionId}`);
    wsRef.current = ws;

    ws.onmessage = (e) => {
      try {
        const ev: WsEvent = JSON.parse(e.data);
        if (!ev.stage_name) return;
        if (ev.type === 'stage_started') {
          updateNodeExecStatus(ev.stage_name, 'running');
        } else if (ev.type === 'stage_completed') {
          updateNodeExecStatus(ev.stage_name, 'success', { execDuration: ev.duration_ms });
        } else if (ev.type === 'stage_failed') {
          updateNodeExecStatus(ev.stage_name, 'failed', { execError: ev.error, execDuration: ev.duration_ms });
        } else if (ev.type === 'stage_retrying') {
          updateNodeExecStatus(ev.stage_name, 'retrying');
        } else if (ev.type === 'agent_token') {
          updateNodeExecStatus(ev.stage_name, 'running', { execTokens: ev.tokens });
        }
      } catch {
        // ignore
      }
    };

    return () => ws.close();
  }, [executionId, updateNodeExecStatus]);
}
