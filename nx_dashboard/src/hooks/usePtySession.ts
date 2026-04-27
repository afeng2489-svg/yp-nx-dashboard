import { useEffect, useRef, useCallback, useState } from 'react';
import type { Terminal } from '@xterm/xterm';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { API_BASE_URL, WS_BASE_URL } from '@/api/constants';

/** True when running inside Tauri (WKWebView / WebView2). */
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

interface PtySessionOptions {
  teamId: string;
  sessionId: string | null;
  terminal: Terminal | null;
  /** 连接失败或 session 不存在时回调，用于上层清除 stale session */
  onSessionLost?: () => void;
}

interface UsePtySessionReturn {
  isConnected: boolean;
  sendTask: (task: string) => void;
  sendInput: (data: string) => void;
  resize: (rows: number, cols: number) => void;
}

export function usePtySession({
  teamId,
  sessionId,
  terminal,
  onSessionLost,
}: PtySessionOptions): UsePtySessionReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  // Keep current sessionId accessible inside stable callbacks
  const sessionIdRef = useRef<string | null>(null);
  sessionIdRef.current = sessionId;
  const onSessionLostRef = useRef(onSessionLost);
  onSessionLostRef.current = onSessionLost;

  // ── Connect / disconnect ───────────────────────────────────────────────────
  useEffect(() => {
    if (!sessionId || !terminal) return;

    if (isTauri) {
      // Tauri IPC path: Rust WS client relays PTY over app events
      let unlistenOutput: UnlistenFn | null = null;
      let unlistenControl: UnlistenFn | null = null;
      let active = true;

      (async () => {
        try {
          // Register listeners BEFORE connecting so no output is missed
          unlistenOutput = await listen<number[]>(`pty-output-${sessionId}`, (evt) => {
            terminal.write(new Uint8Array(evt.payload));
          });

          unlistenControl = await listen<string>(`pty-control-${sessionId}`, (evt) => {
            try {
              const msg = JSON.parse(evt.payload) as { type: string; message?: string };
              if (msg.type === 'ready') {
                setIsConnected(true);
              } else if (msg.type === 'closed') {
                terminal.write('\r\n\x1b[33m[会话已结束]\x1b[0m\r\n');
                setIsConnected(false);
              } else if (msg.type === 'error') {
                terminal.write(`\r\n\x1b[31m[错误] ${msg.message ?? '未知错误'}\x1b[0m\r\n`);
                // 如果是 session 不存在（重启后 localStorage 缓存了旧 ID），清掉 stale 记录让上层重建
                if (msg.message?.includes('not found') || msg.message?.includes('Session')) {
                  onSessionLostRef.current?.();
                }
              }
            } catch {
              /* ignore non-JSON */
            }
          });

          if (!active) {
            unlistenOutput();
            unlistenControl();
            return;
          }

          await invoke('pty_connect', { teamId, sessionId });
        } catch (e) {
          if (active) {
            terminal.write(`\r\n\x1b[31m[IPC连接失败] ${e}\x1b[0m\r\n`);
            // 连接失败说明 session 已不存在，通知上层清除 stale session
            onSessionLostRef.current?.();
          }
        }
      })();

      return () => {
        active = false;
        unlistenOutput?.();
        unlistenControl?.();
        setIsConnected(false);
        invoke('pty_disconnect', { sessionId }).catch(() => {});
      };
    } else {
      // Browser WebSocket path (used with plain `npm run dev` in browser)
      const url = `${WS_BASE_URL}/ws/teams/${teamId}/terminal/${sessionId}`;
      const ws = new WebSocket(url);
      ws.binaryType = 'arraybuffer';
      wsRef.current = ws;

      ws.onmessage = (evt) => {
        if (evt.data instanceof ArrayBuffer) {
          terminal.write(new Uint8Array(evt.data));
        } else if (typeof evt.data === 'string') {
          try {
            const msg = JSON.parse(evt.data) as { type: string; message?: string };
            if (msg.type === 'closed') {
              terminal.write('\r\n\x1b[33m[会话已结束]\x1b[0m\r\n');
              setIsConnected(false);
            } else if (msg.type === 'error') {
              terminal.write(`\r\n\x1b[31m[错误] ${msg.message ?? '未知错误'}\x1b[0m\r\n`);
              // session 不存在（nx_api 重启后 localStorage 缓存的旧 ID 已失效），清掉让上层重建
              if (msg.message?.includes('not found') || msg.message?.includes('Session')) {
                onSessionLostRef.current?.();
              }
            }
          } catch {
            terminal.write(evt.data);
          }
        }
      };

      let connected = false;
      ws.onopen = () => {
        connected = true;
        setIsConnected(true);
      };
      ws.onclose = () => {
        setIsConnected(false);
        // 从未成功建立连接 → session 已不存在，清除 stale 记录
        if (!connected) onSessionLostRef.current?.();
      };
      ws.onerror = () => {
        terminal.write('\r\n\x1b[31m[WebSocket 连接失败]\x1b[0m\r\n');
        setIsConnected(false);
      };

      return () => {
        ws.close();
        wsRef.current = null;
      };
    }
  }, [sessionId, terminal, teamId]);

  // ── Keyboard input → PTY ──────────────────────────────────────────────────
  useEffect(() => {
    if (!terminal || !isConnected) return;

    const disposable = terminal.onData((data) => {
      const sid = sessionIdRef.current;
      if (!sid) return;

      if (isTauri) {
        const bytes = Array.from(new TextEncoder().encode(data));
        invoke('pty_send_input', { sessionId: sid, data: bytes }).catch(() => {});
      } else {
        const ws = wsRef.current;
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(new TextEncoder().encode(data).buffer);
        }
      }
    });

    return () => disposable.dispose();
  }, [terminal, isConnected]);

  // ── Stable callbacks ───────────────────────────────────────────────────────

  const resize = useCallback((rows: number, cols: number) => {
    const sid = sessionIdRef.current;
    if (!sid) return;
    const msg = JSON.stringify({ type: 'resize', rows, cols });
    if (isTauri) {
      invoke('pty_send_control', { sessionId: sid, message: msg }).catch(() => {});
    } else {
      const ws = wsRef.current;
      if (ws?.readyState === WebSocket.OPEN) ws.send(msg);
    }
  }, []);

  const sendTask = useCallback((task: string) => {
    const sid = sessionIdRef.current;
    if (!sid) return;
    const msg = JSON.stringify({ type: 'task', text: task });
    if (isTauri) {
      invoke('pty_send_control', { sessionId: sid, message: msg }).catch(() => {});
    } else {
      const ws = wsRef.current;
      if (ws?.readyState === WebSocket.OPEN) ws.send(msg);
    }
  }, []);

  const sendInput = useCallback((data: string) => {
    const sid = sessionIdRef.current;
    if (!sid) return;
    if (isTauri) {
      const bytes = Array.from(new TextEncoder().encode(data));
      invoke('pty_send_input', { sessionId: sid, data: bytes }).catch(() => {});
    } else {
      const ws = wsRef.current;
      if (ws?.readyState === WebSocket.OPEN) {
        ws.send(new TextEncoder().encode(data).buffer);
      }
    }
  }, []);

  return { isConnected, sendTask, sendInput, resize };
}

// ── Session management API ─────────────────────────────────────────────────

export async function createTerminalSession(
  teamId: string,
  roleId?: string,
  cols?: number,
  rows?: number,
): Promise<string> {
  const res = await fetch(`${API_BASE_URL}/api/v1/teams/${teamId}/terminal`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role_id: roleId, cols, rows }),
  });
  const data = await res.json();
  return data.session_id as string;
}

export async function dispatchTaskToTerminal(
  teamId: string,
  sessionId: string,
  task: string,
): Promise<boolean> {
  const res = await fetch(`${API_BASE_URL}/api/v1/teams/${teamId}/terminal/${sessionId}/task`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ task }),
  });
  const data = await res.json();
  return data.ok as boolean;
}

export async function closeTerminalSession(teamId: string, sessionId: string): Promise<void> {
  await fetch(`${API_BASE_URL}/api/v1/teams/${teamId}/terminal/${sessionId}`, {
    method: 'DELETE',
  });
}
