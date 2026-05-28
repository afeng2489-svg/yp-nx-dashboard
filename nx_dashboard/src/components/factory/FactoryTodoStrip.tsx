import { Link } from 'react-router-dom';
import { AlertCircle, Terminal } from 'lucide-react';
import { useClaudeCliReady } from '@/hooks/useClaudeCliReady';
import { useFactoryApprovalItems } from '@/hooks/useFactoryApprovalItems';
import { useRecentFactoryOutcome } from '@/hooks/useRecentFactoryOutcome';
import { DualEngineStatusBanner } from '@/components/factory/DualEngineStatusBanner';

/** AF-UX-08 精简版 + AF-UX-09 + AF-MM-01：待办条 + 双引擎状态 */
export function FactoryTodoStrip() {
  const approvals = useFactoryApprovalItems();
  const failedOutcome = useRecentFactoryOutcome();
  const { ready: cliReady } = useClaudeCliReady();

  const approvalCount = approvals.filter((i) => i.pauseKind === 'approval').length;
  const failedCount = failedOutcome?.execution.status === 'failed' ? 1 : 0;

  return (
    <div
      className="rounded-xl border border-border/60 bg-muted/30 px-4 py-2.5 space-y-2 text-sm"
      data-testid="factory-todo-strip"
    >
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        {approvalCount > 0 && (
          <Link
            to="/factory?tab=approvals"
            className="inline-flex items-center gap-1.5 text-amber-800 dark:text-amber-200 hover:underline"
          >
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>
              <strong>{approvalCount}</strong> 项待批准
            </span>
          </Link>
        )}
        {failedCount > 0 && (
          <span className="inline-flex items-center gap-1.5 text-destructive">
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>1 项失败 Run 可重试</span>
          </span>
        )}
        {cliReady === false && (
          <Link
            to="/settings/ai"
            className="inline-flex items-center gap-1.5 text-sky-800 dark:text-sky-200 hover:underline"
          >
            <Terminal className="h-4 w-4 shrink-0" />
            <span>未绑定 Claude CLI — 改代码需配置</span>
          </Link>
        )}
      </div>
      <DualEngineStatusBanner compact />
    </div>
  );
}
