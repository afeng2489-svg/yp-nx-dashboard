import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { Play, Square, Plus, Loader2, Terminal as TerminalIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { usePtySession, createTerminalSession, closeTerminalSession } from '../../hooks/usePtySession';
import { useTeamStore } from '../../stores/teamStore';
import type { Role } from '../../stores/teamStore';

// ── 单角色终端（管理一个 xterm + PTY session）────────────────────────────────
interface RoleTerminalTabProps {
  teamId: string;
  roleId: string;
  roleName: string;
  visible: boolean;
}

function RoleTerminalTab({ teamId, roleId, visible }: RoleTerminalTabProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [terminalReady, setTerminalReady] = useState(false);

  const storedSessionId = useTeamStore((s) => s.terminalSessions[teamId]?.[roleId] ?? null);
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

    term.write('\x1b[90m# Claude 终端准备就绪\x1b[0m\r\n');
    term.write('\x1b[90m# 点击"启动"创建此角色的 claude 进程\x1b[0m\r\n\r\n');

    return () => {
      term.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      setTerminalReady(false);
    };
  }, []);

  // 可见时 fit
  useEffect(() => {
    if (!visible) return;
    const id = requestAnimationFrame(() => {
      const term = terminalRef.current;
      if (!fitAddonRef.current || !term) return;
      fitAddonRef.current.fit();
      resize(term.rows, term.cols);
    });
    return () => cancelAnimationFrame(id);
  }, [visible, resize]);

  // 窗口大小变化时 fit
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

  const handleNewSession = useCallback(async () => {
    if (isCreating) return;
    setIsCreating(true);
    try {
      if (sessionId) {
        await closeTerminalSession(teamId, sessionId);
        terminalRef.current?.clear();
        terminalRef.current?.write('\x1b[90m# 已关闭旧会话，正在创建新会话...\x1b[0m\r\n\r\n');
      }

      fitAddonRef.current?.fit();
      const term = terminalRef.current;
      const cols = term?.cols ?? 80;
      const rows = term?.rows ?? 24;

      const id = await createTerminalSession(teamId, roleId, cols, rows);
      setSessionId(id);
      setTerminalSession(teamId, roleId, id);
      terminalRef.current?.write(`\x1b[90m# 会话 ${id.slice(0, 8)} 已创建，正在连接...\x1b[0m\r\n\r\n`);
    } catch (e) {
      terminalRef.current?.write(`\r\n\x1b[31m[错误] 无法创建会话: ${e}\x1b[0m\r\n`);
    } finally {
      setIsCreating(false);
    }
  }, [teamId, roleId, sessionId, isCreating, setTerminalSession]);

  const handleCloseSession = useCallback(async () => {
    if (!sessionId) return;
    await closeTerminalSession(teamId, sessionId);
    setSessionId(null);
    setTerminalSession(teamId, roleId, null);
    terminalRef.current?.write('\r\n\x1b[90m# 会话已关闭\x1b[0m\r\n');
  }, [teamId, roleId, sessionId, setTerminalSession]);

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 工具栏 */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-white/10 flex-shrink-0">
        {sessionId && (
          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${isConnected ? 'bg-green-400' : 'bg-yellow-400 animate-pulse'}`} />
            <span className="text-xs text-white/40 font-mono">{sessionId.slice(0, 8)}</span>
          </div>
        )}
        <div className="flex-1" />
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

      <div ref={containerRef} className="flex-1 overflow-hidden p-1" />
    </div>
  );
}

// ── 多角色终端面板 ─────────────────────────────────────────────────────────────
interface TerminalPanelProps {
  teamId: string;
  roles: Role[];
  workspacePath?: string;
  visible?: boolean;
}

export function TerminalPanel({ teamId, roles, workspacePath, visible = true }: TerminalPanelProps) {
  const [activeRoleId, setActiveRoleId] = useState<string | null>(null);

  // 首次有角色时默认选中第一个
  useEffect(() => {
    if (!activeRoleId && roles.length > 0) {
      setActiveRoleId(roles[0].id);
    }
  }, [roles, activeRoleId]);

  if (roles.length === 0) {
    return (
      <div className="flex flex-col h-full bg-[#0d1117] items-center justify-center">
        <TerminalIcon className="w-8 h-8 text-white/20 mb-2" />
        <p className="text-xs text-white/40 font-mono">暂无角色，请先在团队中添加角色</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 角色 tab 栏 */}
      <div className="flex items-center bg-[#161b22] border-b border-white/10 flex-shrink-0 overflow-x-auto">
        {workspacePath && (
          <span className="px-3 text-xs text-white/30 font-mono flex-shrink-0 border-r border-white/10 py-2">
            {workspacePath}
          </span>
        )}
        {roles.map((role) => (
          <button
            key={role.id}
            onClick={() => setActiveRoleId(role.id)}
            className={cn(
              'px-3 py-2 text-xs font-mono whitespace-nowrap transition-colors border-r border-white/5',
              activeRoleId === role.id
                ? 'text-green-400 bg-[#0d1117] border-b-2 border-b-green-400'
                : 'text-white/40 hover:text-white/70 hover:bg-white/5'
            )}
          >
            {role.name}
          </button>
        ))}
      </div>

      {/* 终端实例（全部挂载，用 hidden 切换可见性，保持 PTY 连接） */}
      {roles.map((role) => (
        <div
          key={role.id}
          className={cn('flex-1 overflow-hidden', activeRoleId !== role.id && 'hidden')}
        >
          <RoleTerminalTab
            teamId={teamId}
            roleId={role.id}
            roleName={role.name}
            visible={visible && activeRoleId === role.id}
          />
        </div>
      ))}
    </div>
  );
}
