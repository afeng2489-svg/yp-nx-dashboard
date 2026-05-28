import { useSearchParams } from 'react-router-dom';
import { CostPage } from '@/pages/CostPage';
import ProcessMonitorPage from '@/pages/ProcessMonitorPage';
import { SprintBoardPage } from '@/pages/SprintBoardPage';
import { ExecutionsPage } from '@/pages/ExecutionsPage';
import { OpsAuditPanel } from '@/components/ops/OpsAuditPanel';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageTabs } from '@/components/ui/PageTabs';

const TABS = [
  { id: 'runs', label: '历史' },
  { id: 'cost', label: '成本' },
  { id: 'processes', label: '进程' },
  { id: 'audit', label: '审计' },
  { id: 'sprint', label: 'Sprint' },
] as const;

/** AF-07 运营中心 — 五 Tab */
export function OpsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get('tab') ?? 'runs';

  return (
    <div className="h-full flex flex-col min-h-0 page-container !py-4 !space-y-4">
      <PageHeader
        title="运营"
        description="Run 历史、成本、进程、审批审计与 Sprint 看板"
      />
      <PageTabs
        items={[...TABS]}
        value={tab}
        onValueChange={(id) => setSearchParams({ tab: id })}
      />
      <div className="flex-1 overflow-auto min-h-0">
        {tab === 'runs' && <ExecutionsPage embedded />}
        {tab === 'cost' && <CostPage embedded />}
        {tab === 'processes' && <ProcessMonitorPage embedded />}
        {tab === 'audit' && <OpsAuditPanel />}
        {tab === 'sprint' && <SprintBoardPage embedded />}
      </div>
    </div>
  );
}
