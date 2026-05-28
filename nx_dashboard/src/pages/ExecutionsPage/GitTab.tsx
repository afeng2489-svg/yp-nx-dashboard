import { useEffect, useState } from 'react';
import { GitBranch, Copy } from 'lucide-react';
import type { BranchInfo } from './types';
import { CommitList } from './GitCommitList';
import { RollbackActions } from './RollbackActions';

export interface GitTabProps {
  executionId: string;
  executionStatus: string;
}

export function GitTab({ executionId, executionStatus }: GitTabProps) {
  const [branchInfo, setBranchInfo] = useState<BranchInfo | null>(null);
  const [commits, setCommits] = useState<
    { hash: string; full_hash: string; message: string; timestamp: string; changed_files: number }[]
  >([]);
  const [expandedCommit, setExpandedCommit] = useState<string | null>(null);
  const [commitDiff, setCommitDiff] = useState<Record<string, string>>({});
  const [prDescription, setPrDescription] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [rollingBack, setRollingBack] = useState(false);
  const [rollbackAction, setRollbackAction] = useState<'revert' | 'keep' | 'branch'>('revert');
  const [copied, setCopied] = useState(false);

  const apiBase = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';

  useEffect(() => {
    setLoading(true);
    fetch(`${apiBase}/api/v1/executions/${executionId}/git`)
      .then((r) => r.json())
      .then((data) => {
        setBranchInfo(data.branch_info);
        setCommits(data.commits || []);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [executionId, apiBase]);

  const loadCommitDiff = (hash: string) => {
    if (commitDiff[hash]) {
      setExpandedCommit(expandedCommit === hash ? null : hash);
      return;
    }
    fetch(`${apiBase}/api/v1/executions/${executionId}/git/commit/${hash}`)
      .then((r) => r.json())
      .then((data) => {
        setCommitDiff((prev) => ({ ...prev, [hash]: data.diff || '' }));
        setExpandedCommit(hash);
      })
      .catch(() => {});
  };

  const handleRollback = async () => {
    if (!branchInfo) return;
    setRollingBack(true);
    try {
      const initialBranch = branchInfo.current_branch || 'main';
      await fetch(`${apiBase}/api/v1/executions/${executionId}/rollback`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          action: rollbackAction,
          initial_branch: initialBranch,
          exec_branch: branchInfo.exec_branch,
        }),
      });
      const resp = await fetch(`${apiBase}/api/v1/executions/${executionId}/git`);
      const data = await resp.json();
      setBranchInfo(data.branch_info);
      setCommits(data.commits || []);
    } catch {
      // ignore rollback error
    } finally {
      setRollingBack(false);
    }
  };

  const handleCopyPr = async () => {
    if (!prDescription) {
      try {
        const resp = await fetch(`${apiBase}/api/v1/executions/${executionId}/pr-description`);
        const data = await resp.json();
        setPrDescription(data.description);
        await navigator.clipboard.writeText(data.description || '');
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch {
        // ignore clipboard error
      }
    } else {
      await navigator.clipboard.writeText(prDescription);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (loading) {
    return <div className="p-4 text-center text-muted-foreground">加载 Git 信息...</div>;
  }

  if (!branchInfo?.is_git_repo) {
    return (
      <div className="text-center py-12">
        <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
          <GitBranch className="w-8 h-8 text-indigo-500" />
        </div>
        <p className="text-muted-foreground">当前工作区不是 Git 仓库</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* 分支信息 */}
      <div className="flex items-center justify-between p-3 bg-accent/50 rounded-lg border border-border">
        <div className="flex items-center gap-2">
          <GitBranch className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium">当前分支:</span>
          <code className="px-2 py-0.5 bg-primary/10 text-primary rounded text-sm">
            {branchInfo.current_branch || 'unknown'}
          </code>
          {branchInfo.current_branch !== branchInfo.exec_branch && (
            <>
              <span className="text-muted-foreground text-sm">执行分支:</span>
              <code className="px-2 py-0.5 bg-purple-500/10 text-purple-500 rounded text-sm">
                {branchInfo.exec_branch}
              </code>
            </>
          )}
        </div>
      </div>

      {/* Commit 列表 */}
      <CommitList
        commits={commits}
        expandedCommit={expandedCommit}
        commitDiff={commitDiff}
        onToggleCommit={loadCommitDiff}
      />

      {/* 操作按钮 */}
      {(executionStatus === 'failed' || executionStatus === 'completed') && (
        <div className="border-t border-border pt-4 space-y-3">
          {executionStatus === 'failed' && (
            <RollbackActions
              rollbackAction={rollbackAction}
              rollingBack={rollingBack}
              onSelectAction={setRollbackAction}
              onRollback={handleRollback}
            />
          )}
          {executionStatus === 'completed' && commits.length > 0 && (
            <button
              onClick={handleCopyPr}
              className="flex items-center gap-2 px-4 py-2 text-sm rounded-lg bg-primary text-white hover:bg-primary/90"
            >
              <Copy className="w-4 h-4" />
              {copied ? '已复制!' : '复制 PR 描述'}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
