import { useEffect } from 'react';
import { GitBranch, Loader2, Terminal } from 'lucide-react';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { detectStackProfile } from '@/data/stackProfile';
import { cn } from '@/lib/utils';

export interface WorkspaceContextBarProps {
  className?: string;
}

/** Phase A — 工作区上下文条：分支 · dirty · 栈 · 脚本 */
export function WorkspaceContextBar({ className }: WorkspaceContextBarProps) {
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const files = useWorkspaceStore((s) => s.files);
  const gitStatus = useWorkspaceStore((s) => s.gitStatus);
  const projectScripts = useWorkspaceStore((s) => s.projectScripts);
  const scriptsLoading = useWorkspaceStore((s) => s.scriptsLoading);
  const fetchGitStatus = useWorkspaceStore((s) => s.fetchGitStatus);
  const fetchProjectScripts = useWorkspaceStore((s) => s.fetchProjectScripts);
  const browseFiles = useWorkspaceStore((s) => s.browseFiles);

  const wsId = currentWorkspace?.id;

  useEffect(() => {
    if (!wsId || !currentWorkspace?.root_path) return;
    void browseFiles();
    void fetchGitStatus();
    void fetchProjectScripts();
  }, [wsId, currentWorkspace?.root_path, browseFiles, fetchGitStatus, fetchProjectScripts]);

  if (!currentWorkspace?.root_path) {
    return (
      <div
        className={cn(
          'rounded-lg border border-dashed border-border/60 px-4 py-2.5 text-xs text-muted-foreground',
          className,
        )}
      >
        选择工作区后显示 Git 分支、技术栈与可用脚本
      </div>
    );
  }

  const stack = detectStackProfile(files);
  const testScript = projectScripts?.scripts.find(
    (s) => s.name === 'test' || s.name === 'build',
  );
  const langLabel =
    stack.language === 'unknown'
      ? projectScripts?.project_type !== 'unknown'
        ? projectScripts?.project_type
        : '未知栈'
      : stack.language;

  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-x-4 gap-y-2 rounded-lg border border-border/60 bg-muted/20 px-4 py-2.5 text-xs',
        className,
      )}
      data-testid="workspace-context-bar"
    >
      <span className="font-medium text-foreground truncate max-w-[140px]" title={currentWorkspace.name}>
        {currentWorkspace.name}
      </span>

      <span className="flex items-center gap-1 text-muted-foreground">
        <GitBranch className="h-3.5 w-3.5 shrink-0" />
        {gitStatus?.branch ?? '—'}
        {gitStatus?.is_dirty && (
          <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-700 dark:text-amber-400">
            未提交
          </span>
        )}
      </span>

      <span className="rounded bg-primary/10 px-2 py-0.5 font-medium text-primary">{langLabel}</span>

      {stack.testCmd && (
        <span className="text-muted-foreground" title="质量门预估">
          门控: {stack.testCmd}
        </span>
      )}

      {scriptsLoading ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
      ) : testScript ? (
        <span className="flex items-center gap-1 text-muted-foreground" title={testScript.command}>
          <Terminal className="h-3.5 w-3.5" />
          {testScript.name}
        </span>
      ) : null}

      {projectScripts && projectScripts.scripts.length > 1 && (
        <span className="text-muted-foreground">{projectScripts.scripts.length} 个脚本</span>
      )}
    </div>
  );
}
