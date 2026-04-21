import { useState, useEffect, useRef, useCallback } from 'react';
import { Play, Square, Trash2, Terminal, ChevronDown, ChevronRight, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useCommandRunner, OutputLine } from '@/hooks/useCommandRunner';
import { API_BASE_URL } from '@/api/constants';

interface ScriptEntry {
  name: string;
  command: string;
}

interface ScriptsResponse {
  project_type: string;
  scripts: ScriptEntry[];
}

const PROJECT_TYPE_LABELS: Record<string, { label: string; color: string }> = {
  node: { label: 'Node.js', color: 'text-green-500' },
  rust: { label: 'Rust', color: 'text-orange-500' },
  python: { label: 'Python', color: 'text-blue-500' },
  make: { label: 'Makefile', color: 'text-yellow-500' },
  go: { label: 'Go', color: 'text-cyan-500' },
  fullstack: { label: '全栈项目', color: 'text-purple-500' },
  unknown: { label: '通用', color: 'text-muted-foreground' },
};

const COMMON_COMMANDS: ScriptEntry[] = [
  { name: 'ls', command: 'ls -la' },
  { name: 'git status', command: 'git status' },
  { name: 'git log', command: 'git log --oneline -10' },
  { name: 'git diff', command: 'git diff' },
  { name: 'pwd', command: 'pwd' },
  { name: 'du -sh', command: 'du -sh .' },
];

export function ProjectRunner() {
  const { currentWorkspace } = useWorkspaceStore();
  const runner = useCommandRunner();

  const [scripts, setScripts] = useState<ScriptEntry[]>([]);
  const [projectType, setProjectType] = useState('unknown');
  const [scriptsLoading, setScriptsLoading] = useState(false);
  const [customCommand, setCustomCommand] = useState('');
  const [showScripts, setShowScripts] = useState(true);
  const [showCommon, setShowCommon] = useState(true);

  const outputEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll output
  useEffect(() => {
    outputEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [runner.output]);

  // Fetch scripts when workspace changes
  useEffect(() => {
    if (!currentWorkspace?.id) {
      setScripts([]);
      setProjectType('unknown');
      return;
    }

    const fetchScripts = async () => {
      setScriptsLoading(true);
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/workspaces/${currentWorkspace.id}/scripts`);
        if (res.ok) {
          const data: ScriptsResponse = await res.json();
          setScripts(data.scripts);
          setProjectType(data.project_type);
        }
      } catch {
        // ignore
      } finally {
        setScriptsLoading(false);
      }
    };

    fetchScripts();
  }, [currentWorkspace?.id]);

  const handleRunScript = useCallback((command: string) => {
    if (!currentWorkspace?.root_path) return;
    runner.execute(command, currentWorkspace.root_path);
  }, [currentWorkspace?.root_path, runner]);

  const handleRunCustom = useCallback(() => {
    if (!customCommand.trim() || !currentWorkspace?.root_path) return;
    runner.execute(customCommand.trim(), currentWorkspace.root_path);
    setCustomCommand('');
  }, [customCommand, currentWorkspace?.root_path, runner]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleRunCustom();
    }
  };

  if (!currentWorkspace) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-center">
        <Terminal className="w-10 h-10 text-muted-foreground/30 mb-3" />
        <p className="text-sm text-muted-foreground">请先选择一个项目</p>
      </div>
    );
  }

  if (!currentWorkspace.root_path) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-center">
        <Terminal className="w-10 h-10 text-muted-foreground/30 mb-3" />
        <p className="text-sm text-muted-foreground">项目未配置根目录</p>
      </div>
    );
  }

  const typeInfo = PROJECT_TYPE_LABELS[projectType] || PROJECT_TYPE_LABELS.unknown;

  return (
    <div className="h-full flex flex-col">
      {/* Project type header */}
      <div className="px-3 py-2 border-b">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className={cn('text-sm font-medium', typeInfo.color)}>
              {typeInfo.label}
            </span>
            {!runner.isConnected && (
              <span className="text-xs text-red-400">未连接</span>
            )}
          </div>
          {runner.isRunning && (
            <button
              onClick={runner.cancel}
              className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-red-500/10 text-red-500 hover:bg-red-500/20 transition-colors"
            >
              <Square className="w-3 h-3" />
              停止
            </button>
          )}
        </div>
      </div>

      {/* Scripts section */}
      <div className="border-b">
        {/* Detected project scripts */}
        {scripts.length > 0 && (
          <>
            <button
              onClick={() => setShowScripts(!showScripts)}
              className="w-full flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              {showScripts ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
              项目脚本
              <span className="text-[10px] opacity-60">({scripts.length})</span>
              {scriptsLoading && <Loader2 className="w-3 h-3 animate-spin ml-1" />}
            </button>
            {showScripts && (
              <div className="px-3 pb-2 flex flex-wrap gap-1.5">
                {scripts.map((script) => (
                  <button
                    key={script.name}
                    onClick={() => handleRunScript(script.command)}
                    disabled={runner.isRunning}
                    className={cn(
                      'flex items-center gap-1 px-2 py-1 text-xs rounded-md border transition-colors',
                      runner.isRunning
                        ? 'opacity-50 cursor-not-allowed'
                        : 'hover:bg-accent hover:border-primary/30'
                    )}
                    title={script.command}
                  >
                    <Play className="w-3 h-3 text-green-500" />
                    {script.name}
                  </button>
                ))}
              </div>
            )}
          </>
        )}

        {/* Common commands — always visible */}
        <button
          onClick={() => setShowCommon(!showCommon)}
          className="w-full flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors"
        >
          {showCommon ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
          常用命令
        </button>
        {showCommon && (
          <div className="px-3 pb-2 flex flex-wrap gap-1.5">
            {COMMON_COMMANDS.map((cmd) => (
              <button
                key={cmd.name}
                onClick={() => handleRunScript(cmd.command)}
                disabled={runner.isRunning}
                className={cn(
                  'flex items-center gap-1 px-2 py-1 text-xs rounded-md border transition-colors',
                  runner.isRunning
                    ? 'opacity-50 cursor-not-allowed'
                    : 'hover:bg-accent hover:border-primary/30'
                )}
                title={cmd.command}
              >
                <Terminal className="w-3 h-3 text-blue-400" />
                {cmd.name}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Custom command input */}
      <div className="px-3 py-2 border-b">
        <div className="flex gap-1.5">
          <div className="flex-1 flex items-center gap-1.5 px-2 py-1 rounded-md border bg-background text-sm">
            <span className="text-muted-foreground">$</span>
            <input
              type="text"
              value={customCommand}
              onChange={(e) => setCustomCommand(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="输入自定义命令..."
              disabled={runner.isRunning}
              className="flex-1 bg-transparent outline-none text-xs placeholder:text-muted-foreground"
            />
          </div>
          <button
            onClick={handleRunCustom}
            disabled={!customCommand.trim() || runner.isRunning}
            className="px-2 py-1 text-xs rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            运行
          </button>
        </div>
      </div>

      {/* Output panel */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {runner.output.length > 0 && (
          <div className="flex items-center justify-between px-3 py-1.5 border-b bg-muted/30">
            <span className="text-xs text-muted-foreground">
              输出日志
              {runner.pid && <span className="ml-1">(PID: {runner.pid})</span>}
            </span>
            <button
              onClick={runner.clear}
              className="p-0.5 rounded hover:bg-accent transition-colors"
              title="清除"
            >
              <Trash2 className="w-3 h-3 text-muted-foreground" />
            </button>
          </div>
        )}
        <div className="flex-1 overflow-y-auto p-2 font-mono text-xs leading-relaxed bg-black/5 dark:bg-black/20">
          {runner.output.length === 0 && !runner.isRunning && (
            <div className="flex flex-col items-center justify-center h-full text-center">
              <Terminal className="w-8 h-8 text-muted-foreground/20 mb-2" />
              <p className="text-xs text-muted-foreground">点击脚本按钮或输入命令运行</p>
            </div>
          )}
          {runner.output.map((line, i) => (
            <OutputLineComponent key={i} line={line} />
          ))}
          {runner.isRunning && (
            <div className="flex items-center gap-1.5 text-muted-foreground mt-1">
              <Loader2 className="w-3 h-3 animate-spin" />
              <span>运行中...</span>
            </div>
          )}
          <div ref={outputEndRef} />
        </div>
      </div>
    </div>
  );
}

function OutputLineComponent({ line }: { line: OutputLine }) {
  return (
    <div
      className={cn(
        'whitespace-pre-wrap break-all',
        line.type === 'stderr' && 'text-red-400',
        line.type === 'system' && 'text-blue-400 font-medium',
        line.type === 'stdout' && 'text-foreground'
      )}
    >
      {line.data}
    </div>
  );
}
