import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { Play, Square, Plus, Loader2, Terminal as TerminalIcon } from 'lucide-react';
import { usePtySession, createTerminalSession, closeTerminalSession } from '../../hooks/usePtySession';
import { useTeamStore } from '../../stores/teamStore';

interface TerminalPanelProps {
  teamId: string;
  /** 当前工作区路径（用于显示） */
  workspacePath?: string;
  /** 当前是否可见（用于触发 fit） */
  visible?: boolean;
}

export function TerminalPanel({ teamId, workspacePath, visible = true }: TerminalPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  // 标记 xterm.js 实例已就绪，触发 usePtySession 重新订阅
  const [terminalReady, setTerminalReady] = useState(false);

  // 从 store 读/写 sessionId，跨 tab 切换持久化
  const storedSessionId = useTeamStore((s) => s.terminalSessions[teamId] ?? null);
  const setTerminalSession = useTeamStore((s) => s.setTerminalSession);
  const [sessionId, setSessionId] = useState<string | null>(storedSessionId);

  const { isConnected, resize } = usePtySession({
    teamId,
    sessionId,
    terminal: terminalReady ? terminalRef.current : null,
  });

  // 初始化 xterm.js
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
    term.open(containerRef.current);
    fitAddon.fit();

    terminalRef.current = term;
    fitAddonRef.current = fitAddon;
    setTerminalReady(true);

    // 欢迎信息
    term.write('\x1b[90m# Claude 终端准备就绪\x1b[0m\r\n');
    term.write('\x1b[90m# 点击"新建会话"启动 claude 进程\x1b[0m\r\n\r\n');

    return () => {
      term.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      setTerminalReady(false);
    };
  }, []);

  // 切换到可见时重新 fit，确保尺寸正确
  useEffect(() => {
    if (!visible) return;
    const id = requestAnimationFrame(() => {
      const fitAddon = fitAddonRef.current;
      const term = terminalRef.current;
      if (!fitAddon || !term) return;
      fitAddon.fit();
      resize(term.rows, term.cols);
    });
    return () => cancelAnimationFrame(id);
  }, [visible, resize]);

  // 自动 fit（窗口大小变化），并同步 PTY 尺寸
  useEffect(() => {
    const obs = new ResizeObserver(() => {
      const fitAddon = fitAddonRef.current;
      const term = terminalRef.current;
      if (!fitAddon || !term) return;
      fitAddon.fit();
      resize(term.rows, term.cols);
    });
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [resize]);

  const handleNewSession = useCallback(async () => {
    if (isCreating) return;
    setIsCreating(true);
    try {
      // 先关闭旧会话
      if (sessionId) {
        await closeTerminalSession(teamId, sessionId);
        terminalRef.current?.clear();
        terminalRef.current?.write('\x1b[90m# 已关闭旧会话，正在创建新会话...\x1b[0m\r\n\r\n');
      }

      // 确保 fit 后再读取尺寸
      fitAddonRef.current?.fit();
      const term = terminalRef.current;
      const cols = term?.cols ?? 80;
      const rows = term?.rows ?? 24;

      const id = await createTerminalSession(teamId, undefined, cols, rows);
      setSessionId(id);
      setTerminalSession(teamId, id);
      terminalRef.current?.write(`\x1b[90m# 会话 ${id.slice(0, 8)} 已创建，正在连接...\x1b[0m\r\n\r\n`);
    } catch (e) {
      terminalRef.current?.write(`\r\n\x1b[31m[错误] 无法创建会话: ${e}\x1b[0m\r\n`);
    } finally {
      setIsCreating(false);
    }
  }, [teamId, sessionId, isCreating]);

  const handleCloseSession = useCallback(async () => {
    if (!sessionId) return;
    await closeTerminalSession(teamId, sessionId);
    setSessionId(null);
    setTerminalSession(teamId, null);
    terminalRef.current?.write('\r\n\x1b[90m# 会话已关闭\x1b[0m\r\n');
  }, [teamId, sessionId, setTerminalSession]);

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 工具栏 */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-white/10 flex-shrink-0">
        <TerminalIcon className="w-4 h-4 text-green-400" />
        <span className="text-xs text-white/60 font-mono flex-1">
          {workspacePath ? workspacePath : 'Claude 终端'}
        </span>

        {/* 会话状态指示 */}
        {sessionId && (
          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${isConnected ? 'bg-green-400' : 'bg-yellow-400 animate-pulse'}`} />
            <span className="text-xs text-white/40 font-mono">{sessionId.slice(0, 8)}</span>
          </div>
        )}

        {/* 关闭会话按钮 */}
        {sessionId && (
          <button
            onClick={handleCloseSession}
            className="flex items-center gap-1 px-2 py-1 rounded text-xs text-red-400 hover:bg-red-500/10 transition-colors"
            title="关闭会话"
          >
            <Square className="w-3 h-3" />
            停止
          </button>
        )}

        {/* 新建会话按钮 */}
        <button
          onClick={handleNewSession}
          disabled={isCreating}
          className="flex items-center gap-1 px-2 py-1 rounded text-xs text-green-400 hover:bg-green-500/10 disabled:opacity-50 transition-colors"
          title="新建终端会话"
        >
          {isCreating ? (
            <Loader2 className="w-3 h-3 animate-spin" />
          ) : sessionId ? (
            <Plus className="w-3 h-3" />
          ) : (
            <Play className="w-3 h-3" />
          )}
          {sessionId ? '新建' : '启动'}
        </button>
      </div>

      {/* 终端区域 */}
      <div ref={containerRef} className="flex-1 overflow-hidden p-1" />
    </div>
  );
}
