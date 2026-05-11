import { useEffect, useRef, useState } from 'react';
import { WS_BASE_URL } from '@/api/constants';
import type { LogEntry, PauseState } from './types';

export interface UseExecutionWebSocketResult {
  logs: LogEntry[];
  wsConnected: boolean;
  currentStage: string | null;
  pauseState: PauseState | null;
  handleResume: (value: string) => void;
}

export function useExecutionWebSocket(executionId: string): UseExecutionWebSocketResult {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [wsConnected, setWsConnected] = useState(false);
  const [currentStage, setCurrentStage] = useState<string | null>(null);
  const [pauseState, setPauseState] = useState<PauseState | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    setLogs([]);
    setWsConnected(false);
    setCurrentStage(null);
    setPauseState(null);

    const ws = new WebSocket(`${WS_BASE_URL}/ws/executions/${executionId}`);
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
          if (data.stage_results?.length > 0) {
            data.stage_results.forEach((sr: { stage_name: string }) => {
              newLogs.push({ type: 'stage', text: `✓ 阶段完成: ${sr.stage_name}` });
            });
          }
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
          setPauseState({
            stage_name: data.stage_name,
            question: data.question,
            options: data.options ?? [],
          });
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
      ws.send(JSON.stringify({ type: 'resume_workflow', execution_id: executionId, value }));
      setPauseState(null);
      setLogs((prev) => [...prev, { type: 'system', text: `▶ 已选择: ${value}` }]);
    }
  };

  return { logs, wsConnected, currentStage, pauseState, handleResume };
}
