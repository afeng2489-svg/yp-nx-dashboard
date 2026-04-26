import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { Square, Terminal as TerminalIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { usePtySession, closeTerminalSession } from '../../hooks/usePtySession';
import { useTeamStore } from '../../stores/teamStore';
import type { Role } from '../../stores/teamStore';

// ── 单终端实例（一个 xterm + 一个 PTY session）───────────────────────────────
interface SingleTerminalProps {
  teamId: string;
  roleId: string;
  sessionId: string;
  visible: boolean;
  onClose: () => void;
}

function SingleTerminal({ teamId, roleId, sessionId, visible, onClose }: SingleTerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [terminalReady, setTerminalReady] = useState(false);

  const setTerminalSession = useTeamStore((s) => s.setTerminalSession);
  const handleSessionLost = useCallback(() => {
    setTerminalSession(teamId, roleId, null);
    onClose();
  }, [teamId, roleId, setTerminalSession, onClose]);

  const { isConnected, resize } = usePtySession({
    teamId,
    sessionId,
    terminal: terminalReady ? terminalRef.current : null,
    onSessionLost: handleSessionLost,
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

  const handleClose = useCallback(async () => {
    await closeTerminalSession(teamId, sessionId);
    setTerminalSession(teamId, roleId, null);
    onClose();
  }, [teamId, sessionId, roleId, setTerminalSession, onClose]);

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 工具栏 */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-white/10 flex-shrink-0">
        <div className="flex items-center gap-1.5">
          <div className={`w-1.5 h-1.5 rounded-full ${isConnected ? 'bg-green-400' : 'bg-yellow-400 animate-pulse'}`} />
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

// ── 统一终端面板 ─────────────────────────────────────────────────────────────
interface TerminalPanelProps {
  teamId: string;
  roles: Role[];
  workspacePath?: string;
  visible?: boolean;
  /** 外部控制：切换到指定角色的终端 tab */
  activeRoleId?: string | null;
}

/** 活跃的终端会话信息 */
interface ActiveSession {
  roleId: string;
  roleName: string;
  sessionId: string;
}

export function TerminalPanel({ teamId, roles, workspacePath, visible = true, activeRoleId: externalActiveRoleId }: TerminalPanelProps) {
  const [activeTabRoleId, setActiveTabRoleId] = useState<string | null>(null);

  // 从 store 读取该团队所有角色的活跃 session
  const terminalSessions = useTeamStore((s) => s.terminalSessions[teamId] ?? {});

  // 构建活跃 session 列表
  const activeSessions: ActiveSession[] = useMemo(() => {
    const sessions: ActiveSession[] = [];
    for (const role of roles) {
      const sid = terminalSessions[role.id];
      if (sid) {
        sessions.push({ roleId: role.id, roleName: role.name, sessionId: sid });
      }
    }
    return sessions;
  }, [roles, terminalSessions]);

  // 外部传入 activeRoleId 时自动切换 tab
  useEffect(() => {
    if (externalActiveRoleId) {
      setActiveTabRoleId(externalActiveRoleId);
    }
  }, [externalActiveRoleId]);

  // 自动选中第一个有 session 的 tab
  useEffect(() => {
    if (activeSessions.length > 0 && !activeSessions.find(s => s.roleId === activeTabRoleId)) {
      setActiveTabRoleId(activeSessions[0].roleId);
    }
  }, [activeSessions, activeTabRoleId]);

  const handleCloseSession = useCallback((roleId: string) => {
    // 如果关闭的是当前 tab，切到下一个
    if (activeTabRoleId === roleId) {
      const remaining = activeSessions.filter(s => s.roleId !== roleId);
      setActiveTabRoleId(remaining.length > 0 ? remaining[0].roleId : null);
    }
  }, [activeTabRoleId, activeSessions]);

  // 没有活跃 session 时显示空状态
  if (activeSessions.length === 0) {
    return (
      <div className="flex flex-col h-full bg-[#0d1117] items-center justify-center">
        <TerminalIcon className="w-8 h-8 text-white/20 mb-2" />
        <p className="text-xs text-white/40 font-mono">发送消息后，终端将自动显示执行过程</p>
      </div>
    );
  }

  const currentSession = activeSessions.find(s => s.roleId === activeTabRoleId) ?? activeSessions[0];

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 活跃角色 tab 栏（只显示有 session 的角色） */}
      {activeSessions.length > 1 && (
        <div className="flex items-center bg-[#161b22] border-b border-white/10 flex-shrink-0 overflow-x-auto">
          {workspacePath && (
            <span className="px-3 text-xs text-white/30 font-mono flex-shrink-0 border-r border-white/10 py-2">
              {workspacePath}
            </span>
          )}
          {activeSessions.map((session) => (
            <button
              key={session.roleId}
              onClick={() => setActiveTabRoleId(session.roleId)}
              className={cn(
                'px-3 py-2 text-xs font-mono whitespace-nowrap transition-colors border-r border-white/5',
                currentSession.roleId === session.roleId
                  ? 'text-green-400 bg-[#0d1117] border-b-2 border-b-green-400'
                  : 'text-white/40 hover:text-white/70 hover:bg-white/5'
              )}
            >
              {session.roleName}
            </button>
          ))}
        </div>
      )}

      {/* 终端实例（全部挂载，用 hidden 切换可见性，保持 PTY 连接） */}
      {activeSessions.map((session) => (
        <div
          key={session.roleId}
          className={cn('flex-1 overflow-hidden', currentSession.roleId !== session.roleId && 'hidden')}
        >
          <SingleTerminal
            teamId={teamId}
            roleId={session.roleId}
            sessionId={session.sessionId}
            visible={visible && currentSession.roleId === session.roleId}
            onClose={() => handleCloseSession(session.roleId)}
          />
        </div>
      ))}
    </div>
  );
}
