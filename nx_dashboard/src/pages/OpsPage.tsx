import { useSearchParams } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { CostPage } from '@/pages/CostPage';
import { TeamSessionsPage } from '@/pages/TeamSessionsPage';
import ProcessMonitorPage from '@/pages/ProcessMonitorPage';
import { SprintBoardPage } from '@/pages/SprintBoardPage';

const TABS = [
  { id: 'cost', label: '成本' },
  { id: 'history', label: '历史' },
  { id: 'processes', label: '进程' },
  { id: 'sprint', label: 'Sprint' },
] as const;

export function OpsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get('tab') ?? 'cost';

  return (
    <div className="h-full flex flex-col min-h-0">
      <div className="px-4 sm:px-6 pt-4 border-b border-border/50 shrink-0">
        <h1 className="text-xl font-semibold mb-3">运营</h1>
        <div className="flex gap-1 overflow-x-auto">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setSearchParams({ tab: t.id })}
              className={cn(
                'px-3 py-2 text-sm border-b-2 -mb-px whitespace-nowrap',
                tab === t.id
                  ? 'border-primary text-primary font-medium'
                  : 'border-transparent text-muted-foreground',
              )}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>
      <div className="flex-1 overflow-auto min-h-0">
        {tab === 'cost' && <CostPage />}
        {tab === 'history' && <TeamSessionsPage />}
        {tab === 'processes' && <ProcessMonitorPage />}
        {tab === 'sprint' && <SprintBoardPage />}
      </div>
    </div>
  );
}
