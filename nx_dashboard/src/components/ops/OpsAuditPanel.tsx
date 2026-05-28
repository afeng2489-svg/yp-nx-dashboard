import { useEffect } from 'react';
import { useExecutionStore, type ApprovalEvent } from '@/stores/executionStore';
import { Loader2 } from 'lucide-react';

/** 运营 — 审批/Run 审计（approval_events） */
export function OpsAuditPanel() {
  const { executions, fetchExecutions, loading } = useExecutionStore();

  useEffect(() => {
    void fetchExecutions();
  }, [fetchExecutions]);

  const rows: Array<ApprovalEvent & { execution_id: string; workflow_id: string }> = [];
  for (const ex of executions) {
    for (const ev of ex.approval_events ?? []) {
      rows.push({
        ...ev,
        execution_id: ex.id,
        workflow_id: ex.workflow_id,
      });
    }
  }
  rows.sort((a, b) => (b.decided_at ?? '').localeCompare(a.decided_at ?? ''));

  if (loading && executions.length === 0) {
    return (
      <div className="flex justify-center py-16">
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="py-16 text-center text-muted-foreground text-sm">
        暂无审批审计记录。工厂台 Run 经「交付审批」后会出现此处。
      </div>
    );
  }

  return (
    <div className="space-y-4 pb-4">
      <p className="text-sm text-muted-foreground">共 {rows.length} 条审批记录</p>
      <div className="rounded-xl border border-border/50 overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b bg-accent/30 text-muted-foreground">
              <th className="text-left px-4 py-2">时间</th>
              <th className="text-left px-4 py-2">Run</th>
              <th className="text-left px-4 py-2">Stage</th>
              <th className="text-left px-4 py-2">结果</th>
              <th className="text-left px-4 py-2">备注</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, i) => (
              <tr key={`${row.execution_id}-${i}`} className="border-b border-border/30 last:border-0">
                <td className="px-4 py-2 whitespace-nowrap text-muted-foreground">
                  {row.decided_at ? new Date(row.decided_at).toLocaleString('zh-CN') : '—'}
                </td>
                <td className="px-4 py-2 font-mono text-xs truncate max-w-[140px]" title={row.execution_id}>
                  {row.execution_id.slice(0, 8)}…
                </td>
                <td className="px-4 py-2">{row.stage_name ?? '—'}</td>
                <td className="px-4 py-2">
                  <span
                    className={
                      row.approved
                        ? 'text-emerald-600 dark:text-emerald-400'
                        : 'text-amber-600 dark:text-amber-400'
                    }
                  >
                    {row.approved ? '通过' : '驳回'}
                  </span>
                </td>
                <td className="px-4 py-2 text-muted-foreground truncate max-w-xs">
                  {row.comment || '—'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
