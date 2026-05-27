import { useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { FactoryConsoleTab } from '@/components/factory/FactoryConsoleTab';
import { FactoryApprovalsTab } from '@/components/factory/FactoryApprovalsTab';
import { FactoryDeliverablesTab } from '@/components/factory/FactoryDeliverablesTab';
import { SoloTeamWizardBanner } from '@/components/factory/SoloTeamWizardBanner';
import { ExecutionsPage } from '@/pages/ExecutionsPage';
import { useExecutionStore } from '@/stores/executionStore';

const TABS = [
  { id: 'console', label: 'Console' },
  { id: 'runs', label: 'Runs' },
  { id: 'approvals', label: 'Approvals' },
  { id: 'deliverables', label: 'Deliverables' },
] as const;

type FactoryTab = (typeof TABS)[number]['id'];

export function FactoryPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = (searchParams.get('tab') as FactoryTab) || 'console';
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);

  useEffect(() => {
    fetchExecutions();
  }, [fetchExecutions]);

  const setTab = (id: FactoryTab) => {
    setSearchParams(id === 'console' ? {} : { tab: id });
  };

  return (
    <div className="page-container space-y-6 pb-8">
      <div>
        <h1 className="text-2xl sm:text-3xl font-bold tracking-tight">
          <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
            工厂台
          </span>
        </h1>
        <p className="text-muted-foreground text-sm mt-1">一句话 → Run → diff → 审批</p>
      </div>

      <SoloTeamWizardBanner />

      <div className="flex gap-1 border-b border-border/60 overflow-x-auto">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={cn(
              'px-4 py-2.5 text-sm font-medium whitespace-nowrap border-b-2 -mb-px transition-colors',
              tab === t.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground',
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'console' && (
        <FactoryConsoleTab onRunStarted={() => setTab('runs')} />
      )}
      {tab === 'runs' && <ExecutionsPage />}
      {tab === 'approvals' && <FactoryApprovalsTab />}
      {tab === 'deliverables' && <FactoryDeliverablesTab />}
    </div>
  );
}
