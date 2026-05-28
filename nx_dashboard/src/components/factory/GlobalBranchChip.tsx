import { useEffect } from 'react';
import { GitBranch } from 'lucide-react';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { cn } from '@/lib/utils';

/** 顶栏 Git 分支（工作区） */
export function GlobalBranchChip() {
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const gitStatus = useWorkspaceStore((s) => s.gitStatus);
  const fetchGitStatus = useWorkspaceStore((s) => s.fetchGitStatus);

  useEffect(() => {
    if (!currentWorkspace?.id) return;
    void fetchGitStatus();
    const t = window.setInterval(() => void fetchGitStatus(), 60_000);
    return () => window.clearInterval(t);
  }, [currentWorkspace?.id, fetchGitStatus]);

  if (!currentWorkspace?.root_path) return null;

  const branch = gitStatus?.branch ?? '—';
  const dirty = gitStatus?.is_dirty;

  return (
    <span
      className="hidden md:inline-flex items-center gap-1 text-[10px] sm:text-xs px-2 py-0.5 rounded-full border border-border/60 bg-muted/30 max-w-[140px]"
      title={
        gitStatus
          ? `${branch}${dirty ? ' · 有未提交变更' : ''}${gitStatus.ahead || gitStatus.behind ? ` · ↑${gitStatus.ahead} ↓${gitStatus.behind}` : ''}`
          : 'Git 分支'
      }
    >
      <GitBranch className="w-3 h-3 shrink-0 opacity-70" />
      <span className="truncate font-mono">{branch}</span>
      {dirty && (
        <span className={cn('w-1.5 h-1.5 rounded-full shrink-0 bg-amber-500')} aria-label="dirty" />
      )}
    </span>
  );
}
