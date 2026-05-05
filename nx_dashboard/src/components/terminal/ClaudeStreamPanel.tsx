import { useEffect, useRef, useState } from 'react';
import { useClaudeStream } from '@/hooks/useClaudeStream';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Send, Square, Trash2, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';

import '@xterm/xterm/css/xterm.css';

interface ClaudeStreamPanelProps {
  className?: string;
  initialPrompt?: string;
  workingDirectory?: string;
  onClose?: () => void;
}

export function ClaudeStreamPanel({
  className,
  initialPrompt,
  workingDirectory,
  onClose,
}: ClaudeStreamPanelProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [input, setInput] = useState(initialPrompt || '');
  const [inputDir, setInputDir] = useState(workingDirectory || '');

  const {
    isConnected,
    isExecuting,
    output,
    error,
    execute,
    cancel,
    clear,
    error: connectionError,
  } = useClaudeStream({
    onOutput: (line, isError) => {
      xtermRef.current?.write(isError ? `\x1b[31m${line}\r\n\x1b[0m` : `${line}\r\n`);
    },
  });

  // Initialize terminal
  useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#ffffff',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78',
      },
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(terminalRef.current);
    requestAnimationFrame(() => {
      fitAddon.fit();
    });

    xtermRef.current = terminal;
    fitAddonRef.current = fitAddon;

    // Write welcome message
    terminal.writeln('\x1b[36m[Claude CLI Stream]\x1b[0m 连接中...');
    terminal.writeln('');

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        // Ignore fit errors during rapid resize
      }
    });
    resizeObserver.observe(terminalRef.current);

    return () => {
      resizeObserver.disconnect();
      terminal.dispose();
    };
  }, []);

  // Update connection status
  useEffect(() => {
    if (!xtermRef.current) return;

    if (isConnected) {
      xtermRef.current.writeln('\x1b[32m[已连接]\x1b[0m Claude CLI 流式输出就绪');
      xtermRef.current.writeln('');
    } else if (!connectionError) {
      xtermRef.current.writeln('\x1b[33m[连接中...]\x1b[0m');
    }
  }, [isConnected, connectionError]);

  // Write output lines
  useEffect(() => {
    if (!xtermRef.current || output.length === 0) return;
    const lastLine = output[output.length - 1];
    xtermRef.current.write(`${lastLine}\r\n`);
  }, [output]);

  // Write errors
  useEffect(() => {
    if (!xtermRef.current || !error) return;
    xtermRef.current.writeln(`\x1b[31m[错误] ${error}\x1b[0m`);
  }, [error]);

  // Handle initial prompt
  useEffect(() => {
    if (initialPrompt && isConnected && !isExecuting) {
      setInput(initialPrompt);
      execute(initialPrompt, workingDirectory);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialPrompt, isConnected, isExecuting]);

  const handleExecute = () => {
    if (!input.trim() || isExecuting) return;
    clear();
    xtermRef.current?.writeln(`\x1b[36m[执行]\x1b[0m ${input}`);
    xtermRef.current?.writeln('');
    execute(input, inputDir || undefined);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleExecute();
    }
  };

  return (
    <div className={cn('flex flex-col bg-[#1e1e1e] rounded-lg overflow-hidden', className)}>
      {/* Terminal output */}
      <div ref={terminalRef} className="flex-1 min-h-[200px]" />

      {/* Input area */}
      <div className="p-3 bg-[#252526] border-t border-[#3c3c3c]">
        {/* Working directory */}
        <div className="flex items-center gap-2 mb-2">
          <span className="text-xs text-gray-500">工作目录:</span>
          <input
            type="text"
            value={inputDir}
            onChange={(e) => setInputDir(e.target.value)}
            placeholder="留空使用项目目录"
            className="flex-1 px-2 py-1 text-xs bg-[#1e1e1e] border border-[#3c3c3c] rounded text-gray-300 focus:outline-none focus:border-[#007acc]"
            disabled={isExecuting}
          />
        </div>

        {/* Command input */}
        <div className="flex items-center gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="输入要执行的命令..."
            className="flex-1 px-3 py-2 text-sm bg-[#1e1e1e] border border-[#3c3c3c] rounded text-gray-300 focus:outline-none focus:border-[#007acc] resize-none"
            rows={2}
            disabled={isExecuting || !isConnected}
          />
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-2 mt-2">
          {isExecuting ? (
            <button
              onClick={cancel}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors"
            >
              <Square className="w-3.5 h-3.5" />
              停止
            </button>
          ) : (
            <button
              onClick={handleExecute}
              disabled={!input.trim() || !isConnected}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-[#007acc] hover:bg-[#007acc]/80 text-white rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Send className="w-3.5 h-3.5" />
              执行
            </button>
          )}

          <button
            onClick={() => {
              clear();
              xtermRef.current?.clear();
            }}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-400 hover:text-gray-300 hover:bg-[#3c3c3c] rounded transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
            清空
          </button>

          {onClose && (
            <button
              onClick={onClose}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-400 hover:text-gray-300 hover:bg-[#3c3c3c] rounded transition-colors"
            >
              关闭
            </button>
          )}

          <div className="flex-1" />

          {/* Status indicator */}
          <div className="flex items-center gap-2 text-xs text-gray-500">
            {isExecuting ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin text-blue-400" />
                <span className="text-blue-400">执行中...</span>
              </>
            ) : isConnected ? (
              <>
                <span className="w-2 h-2 rounded-full bg-green-500" />
                <span>就绪</span>
              </>
            ) : (
              <>
                <span className="w-2 h-2 rounded-full bg-yellow-500 animate-pulse" />
                <span>连接中...</span>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
