import { useEffect, useRef, useCallback, useState } from 'react';
import type { Terminal } from '@xterm/xterm';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WS_BASE_URL } from '@/api/constants';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

interface ShellPtyOptions {
  terminal: Terminal | null;
  cwd?: string;
  enabled?: boolean;
  rows?: number;
  cols?: number;
  /** 递增此值以重建 PTY 会话 */
  sessionKey?: number;
  onSessionEnded?: (exitCode: number) => void;
}

interface ShellPtyReturn {
  isConnected: boolean;
  sessionEnded: boolean;
  resize: (rows: number, cols: number) => void;
}

/** 工作区 Shell 终端：Tauri 本地 PTY 或浏览器 WebSocket 二进制直通 */
export function useShellPtySession({
  terminal,
  cwd,
  enabled = true,
  rows = 24,
  cols = 80,
  sessionKey = 0,
  onSessionEnded,
}: ShellPtyOptions): ShellPtyReturn {
  const [isConnected, setIsConnected] = useState(false);
  const [sessionEnded, setSessionEnded] = useState(false);
  const sessionIdRef = useRef<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const onSessionEndedRef = useRef(onSessionEnded);
  onSessionEndedRef.current = onSessionEnded;

  const resize = useCallback((r: number, c: number) => {
    const sid = sessionIdRef.current;
    const msg = JSON.stringify({ type: 'resize', rows: r, cols: c });
    if (isTauri && sid) {
      invoke('pty_send_control', { sessionId: sid, message: msg }).catch(() => {});
    } else {
      const ws = wsRef.current;
      if (ws?.readyState === WebSocket.OPEN) ws.send(msg);
    }
  }, []);

  useEffect(() => {
    if (!terminal || !enabled) return;

    let active = true;
    let unlistenOutput: UnlistenFn | null = null;
    let unlistenControl: UnlistenFn | null = null;

    const cleanup = () => {
      active = false;
      unlistenOutput?.();
      unlistenControl?.();
      unlistenOutput = null;
      unlistenControl = null;
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
      const sid = sessionIdRef.current;
      sessionIdRef.current = null;
      if (sid && isTauri) {
        invoke('pty_disconnect', { sessionId: sid }).catch(() => {});
      }
      setIsConnected(false);
    };

    const connectTauri = async () => {
      try {
        const sessionId = await invoke<string>('pty_spawn_shell', {
          workingDir: cwd ?? null,
          rows,
          cols,
        });
        if (!active) {
          invoke('pty_disconnect', { sessionId }).catch(() => {});
          return;
        }

        sessionIdRef.current = sessionId;

        unlistenOutput = await listen<number[]>(`pty-output-${sessionId}`, (evt) => {
          terminal.write(new Uint8Array(evt.payload));
        });

        unlistenControl = await listen<string>(`pty-control-${sessionId}`, (evt) => {
          try {
            const msg = JSON.parse(evt.payload) as {
              type: string;
              exit_code?: number;
              message?: string;
            };
            if (msg.type === 'closed' || msg.type === 'session_ended') {
              setIsConnected(false);
              setSessionEnded(true);
              onSessionEndedRef.current?.(msg.exit_code ?? 0);
            } else if (msg.type === 'error') {
              terminal.write(`\r\n\x1b[31m[错误] ${msg.message ?? '未知'}\x1b[0m\r\n`);
            }
          } catch {
            /* ignore */
          }
        });

        if (!active) return;

        await invoke('pty_start', { sessionId });
        setIsConnected(true);
        setSessionEnded(false);
      } catch (e) {
        if (active) {
          terminal.write(`\r\n\x1b[31m[终端启动失败] ${e}\x1b[0m\r\n`);
          setSessionEnded(true);
        }
      }
    };

    const connectBrowser = () => {
      const wsUrl = cwd
        ? `${WS_BASE_URL}/ws/terminal?cwd=${encodeURIComponent(cwd)}`
        : `${WS_BASE_URL}/ws/terminal`;

      const ws = new WebSocket(wsUrl);
      ws.binaryType = 'arraybuffer';
      wsRef.current = ws;

      ws.onopen = () => {
        if (!active) return;
        setIsConnected(true);
        setSessionEnded(false);
        resize(terminal.rows, terminal.cols);
      };

      ws.onmessage = (evt) => {
        if (!active) return;
        if (evt.data instanceof ArrayBuffer) {
          terminal.write(new Uint8Array(evt.data));
        } else if (typeof evt.data === 'string') {
          try {
            const msg = JSON.parse(evt.data) as {
              type: string;
              exit_code?: number;
              message?: string;
            };
            if (msg.type === 'ready') {
              setIsConnected(true);
              setSessionEnded(false);
            } else if (msg.type === 'session_ended') {
              setIsConnected(false);
              setSessionEnded(true);
              onSessionEndedRef.current?.(msg.exit_code ?? 0);
            } else if (msg.type === 'error') {
              terminal.write(`\r\n\x1b[31m[错误] ${msg.message ?? '未知'}\x1b[0m\r\n`);
            }
          } catch {
            terminal.write(evt.data);
          }
        }
      };

      ws.onclose = () => {
        if (!active) return;
        setIsConnected(false);
      };

      ws.onerror = () => {
        if (!active) return;
        setIsConnected(false);
        setSessionEnded(true);
      };
    };

    setSessionEnded(false);
    if (isTauri) {
      void connectTauri();
    } else {
      connectBrowser();
    }

    return cleanup;
  }, [terminal, cwd, enabled, rows, cols, sessionKey, resize]);

  // 键盘输入 → PTY（raw 模式，逐字节）
  useEffect(() => {
    if (!terminal || !isConnected || sessionEnded) return;

    const disposable = terminal.onData((data) => {
      const sid = sessionIdRef.current;
      const bytes = new TextEncoder().encode(data);

      if (isTauri && sid) {
        invoke('pty_send_input', { sessionId: sid, data: Array.from(bytes) }).catch(() => {});
      } else {
        const ws = wsRef.current;
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(bytes.buffer);
        }
      }
    });

    return () => disposable.dispose();
  }, [terminal, isConnected, sessionEnded]);

  return { isConnected, sessionEnded, resize };
}
