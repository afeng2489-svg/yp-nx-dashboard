import { Link } from 'react-router-dom';
import { CheckCircle2 } from 'lucide-react';
import { ApprovalPanel } from '@/components/factory/ApprovalPanel';
import { useFactoryApprovalItems } from '@/hooks/useFactoryApprovalItems';

/** Console 底部：待审批前 2 条摘要 */
export function ConsoleApprovalSummary() {
  const items = useFactoryApprovalItems();
  const approvalOnly = items.filter((i) => i.pauseKind === 'approval');
  const preview = approvalOnly.slice(0, 2);

  if (preview.length === 0) return null;

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 text-amber-500" />
          待审批 ({approvalOnly.length})
        </h3>
        <Link to="/factory?tab=approvals" className="text-xs text-primary hover:underline">
          查看全部
        </Link>
      </div>
      {preview.map((item) => (
        <ApprovalPanel
          key={item.executionId}
          executionId={item.executionId}
          stageName={item.stageName}
          question={item.question}
          compact
        />
      ))}
    </section>
  );
}
