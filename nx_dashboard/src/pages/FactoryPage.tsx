import { useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { FactoryConsoleTab } from '@/components/factory/FactoryConsoleTab';
import { FactoryApprovalsTab } from '@/components/factory/FactoryApprovalsTab';
import { FactoryDeliverablesTab } from '@/components/factory/FactoryDeliverablesTab';
import { SoloTeamWizardBanner } from '@/components/factory/SoloTeamWizardBanner';
import { ExecutionsPage } from '@/pages/ExecutionsPage';
import { useExecutionStore } from '@/stores/executionStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { DEFAULT_LAYOUT_VARIANT } from '@/data/layoutVariants';
import { recordFactoryEvent } from '@/services/factoryMetrics';
import CrashRecoveryDialog from '@/components/team/CrashRecoveryDialog';
import { useProjectStore } from '@/stores/projectStore';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageTabs } from '@/components/ui/PageTabs';

const TABS = [
  { id: 'console', label: '控制台' },
  { id: 'runs', label: '运行中' },
  { id: 'approvals', label: '待审批' },
  { id: 'deliverables', label: '产物' },
] as const;

type FactoryTab = (typeof TABS)[number]['id'];

export function FactoryPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = (searchParams.get('tab') as FactoryTab) || 'console';
  const intentParam = searchParams.get('intent') ?? undefined;
  const sprintIdParam = searchParams.get('sprint_id') ?? undefined;
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);
  const currentProject = useProjectStore((s) => s.currentProject);
  const layoutVariant = useSettingsStore((s) => s.layout.variant ?? DEFAULT_LAYOUT_VARIANT);
  const consoleVariant = layoutVariant === 'refined' ? 'guided-refined' : 'full';

  useEffect(() => {
    fetchExecutions();
    void recordFactoryEvent('factory_opened');
  }, [fetchExecutions]);

  const setTab = (id: FactoryTab) => {
    setSearchParams(id === 'console' ? {} : { tab: id });
  };

  return (
    <div className="page-container space-y-6 pb-8">
      <PageHeader title="工厂台" description="一句话 → Run → diff → 审批" />

      <SoloTeamWizardBanner />

      <CrashRecoveryDialog
        projectId={currentProject?.id}
        onResumed={() => void fetchExecutions()}
      />

      <PageTabs
        items={[...TABS]}
        value={tab}
        onValueChange={(id) => setTab(id as FactoryTab)}
      />

      {tab === 'console' && (
        <FactoryConsoleTab
          variant={consoleVariant}
          initialIntent={intentParam}
          sprintId={sprintIdParam}
          onRunStarted={() => setTab('runs')}
        />
      )}
      {tab === 'runs' && (
        <ExecutionsPage embedded detailMode="context-only" showFilters runsOnly />
      )}
      {tab === 'approvals' && <FactoryApprovalsTab />}
      {tab === 'deliverables' && <FactoryDeliverablesTab />}
    </div>
  );
}
