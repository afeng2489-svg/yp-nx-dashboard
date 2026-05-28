import { useState } from 'react';
import { PageGuideBanner } from '@/components/ui/PageGuideBanner';
import { WORKFLOW_OPERATIONS } from './constants';

export function WorkflowOperationsGuide() {
  const [isExpanded, setIsExpanded] = useState(true);

  if (!isExpanded) return null;

  return (
    <PageGuideBanner title="工作流操作说明" onDismiss={() => setIsExpanded(false)}>
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
        {WORKFLOW_OPERATIONS.map((op) => (
          <div key={op.key} className="text-sm">
            <p className="font-medium text-foreground">{op.action}</p>
            <p className="text-xs text-muted-foreground mt-0.5 leading-snug">{op.desc}</p>
          </div>
        ))}
      </div>
    </PageGuideBanner>
  );
}
