import { ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CommitInfo } from './types';

export function CommitList({
  commits,
  expandedCommit,
  commitDiff,
  onToggleCommit,
}: {
  commits: CommitInfo[];
  expandedCommit: string | null;
  commitDiff: Record<string, string>;
  onToggleCommit: (hash: string) => void;
}) {
  if (commits.length === 0) {
    return (
      <div className="text-center py-8">
        <p className="text-muted-foreground">暂无 commit 记录</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-muted-foreground">Commits ({commits.length})</h3>
      {commits.map((commit) => (
        <div key={commit.hash} className="border border-border rounded-lg overflow-hidden">
          <button
            onClick={() => onToggleCommit(commit.hash)}
            className="w-full p-3 text-left hover:bg-accent/50 transition-colors flex items-center gap-3"
          >
            <code className="text-xs text-primary bg-primary/10 px-2 py-0.5 rounded">
              {commit.hash}
            </code>
            <span className="flex-1 text-sm font-medium">{commit.message}</span>
            <span className="text-xs text-muted-foreground">{commit.changed_files} 文件</span>
            <ChevronRight
              className={cn(
                'w-4 h-4 text-muted-foreground transition-transform',
                expandedCommit === commit.hash && 'rotate-90',
              )}
            />
          </button>
          {expandedCommit === commit.hash && commitDiff[commit.hash] && (
            <div className="border-t border-border">
              <pre className="p-3 text-xs overflow-x-auto bg-card font-mono text-foreground/80 max-h-96 overflow-y-auto">
                {commitDiff[commit.hash]}
              </pre>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
