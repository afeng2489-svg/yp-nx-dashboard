import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { Square, Terminal as TerminalIcon } from 'lucide-react';
import { useTeamPtySession } from '@/hooks/useTeamPtySession';

interface TeamSessionTerminalProps {
  sessionId: string;
  onClose: () => void;
}

export function TeamSessionTerminal({ sessionId, onClose }: TeamSessionTerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [terminalReady, setTerminalReady] = useState(false);

  const { isConnected, resize, disconnect } = useTeamPtySession({
    sessionId: terminalReady ? sessionId : null,
    terminal: terminalReady ? terminalRef.current : null,
  });

  // Init xterm.js
  useEffect(() => {
    if (!containerRef.current || terminalRef.current) return;

    const term = new Terminal({
      theme: {
        background: '#0d1117',
        foreground: '#e6edf3',
        cursor: '#e6edf3',
        selectionBackground: '#264f78',
        black: '#484f58',
        red: '#ff7b72',
        green: '#3fb950',
        yellow: '#d29922',
        blue: '#58a6ff',
        magenta: '#bc8cff',
        cyan: '#39c5cf',
        white: '#b1bac4',
        brightBlack: '#6e7681',
        brightRed: '#ffa198',
        brightGreen: '#56d364',
        brightYellow: '#e3b341',
        brightBlue: '#79c0ff',
        brightMagenta: '#d2a8ff',
        brightCyan: '#56d4dd',
        brightWhite: '#f0f6fc',
      },
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
      fontSize: 13,
      lineHeight: 1.4,
      cursorBlink: true,
      scrollback: 5000,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    // Esc to blur terminal, return focus to page
    term.attachCustomKeyEventHandler((e) => {
      if (e.key === 'Escape' && e.type === 'keydown') {
        term.blur();
        return false;
      }
      return true;
    });

    term.open(containerRef.current);
    fitAddon.fit();
    term.focus();

    terminalRef.current = term;
    fitAddonRef.current = fitAddon;
    setTerminalReady(true);

    return () => {
      term.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      setTerminalReady(false);
    };
  }, []);

  // Focus when connected
  useEffect(() => {
    if (isConnected && terminalRef.current) {
      terminalRef.current.focus();
    }
  }, [isConnected]);

  // ResizeObserver for fit + notify PTY
  useEffect(() => {
    const obs = new ResizeObserver(() => {
      const term = terminalRef.current;
      if (!fitAddonRef.current || !term) return;
      fitAddonRef.current.fit();
      resize(term.rows, term.cols);
    });
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [resize]);

  const handleClose = useCallback(() => {
    disconnect();
    onClose();
  }, [disconnect, onClose]);

  return (
    <div className="flex flex-col h-full bg-[#0d1117] rounded-lg border border-white/10 overflow-hidden shadow-2xl">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-white/10 flex-shrink-0">
        <div className="flex items-center gap-1.5">
          <TerminalIcon className="w-3.5 h-3.5 text-white/50" />
          <div
            className={`w-1.5 h-1.5 rounded-full ${isConnected ? 'bg-green-400' : 'bg-yellow-400 animate-pulse'}`}
          />
          <span className="text-xs text-white/40 font-mono">{sessionId.slice(0, 8)}</span>
        </div>
        <div className="flex-1" />
        <button
          onClick={handleClose}
          className="flex items-center gap-1 px-2 py-1 rounded text-xs text-red-400 hover:bg-red-500/10 transition-colors"
          title="关闭会话"
        >
          <Square className="w-3 h-3" />
          停止
        </button>
      </div>

      <div ref={containerRef} className="flex-1 overflow-hidden p-1" />
    </div>
  );
}
