import { useEffect, useRef, useCallback, useState } from 'react';
import type { Terminal } from '@xterm/xterm';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface UseTeamPtySessionOptions {
  sessionId: string | null;
  terminal: Terminal | null;
}

interface UseTeamPtySessionReturn {
  isConnected: boolean;
  resize: (rows: number, cols: number) => void;
  disconnect: () => void;
}

export function useTeamPtySession({
  sessionId,
  terminal,
}: UseTeamPtySessionOptions): UseTeamPtySessionReturn {
  const [isConnected, setIsConnected] = useState(false);
  const sessionIdRef = useRef<string | null>(null);
  sessionIdRef.current = sessionId;

  useEffect(() => {
    if (!sessionId || !terminal) return;

    let unlistenOutput: UnlistenFn | null = null;
    let unlistenControl: UnlistenFn | null = null;
    let active = true;

    (async () => {
      try {
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

        // Signal backend that frontend listeners are ready — unblocks the PTY reader thread
        invoke('pty_start', { sessionId }).catch(() => {});
        setIsConnected(true);
      } catch (e) {
        if (active) {
          terminal.write(`\r\n\x1b[31m[IPC连接失败] ${e}\x1b[0m\r\n`);
        }
      }
    })();

    return () => {
      active = false;
      unlistenOutput?.();
      unlistenControl?.();
      setIsConnected(false);
    };
  }, [sessionId, terminal]);

  // Keyboard input → PTY
  useEffect(() => {
    if (!terminal || !isConnected) return;

    const disposable = terminal.onData((data) => {
      const sid = sessionIdRef.current;
      if (!sid) return;
      const bytes = Array.from(new TextEncoder().encode(data));
      invoke('pty_send_input', { sessionId: sid, data: bytes }).catch(() => {});
    });

    return () => disposable.dispose();
  }, [terminal, isConnected]);

  const resize = useCallback((rows: number, cols: number) => {
    const sid = sessionIdRef.current;
    if (!sid) return;
    const msg = JSON.stringify({ type: 'resize', rows, cols });
    invoke('pty_send_control', { sessionId: sid, message: msg }).catch(() => {});
  }, []);

  const disconnect = useCallback(() => {
    const sid = sessionIdRef.current;
    if (!sid) return;
    invoke('pty_disconnect', { sessionId: sid }).catch(() => {});
  }, []);

  return { isConnected, resize, disconnect };
}
