import { useSearchParams } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { WorkflowsPage } from '@/pages/WorkflowsPage';
import { RolesPage } from '@/pages/RolesPage';
import { SkillsPage } from '@/pages/SkillsPage';
import { WisdomPage } from '@/pages/WisdomPage';

const TABS = [
  { id: 'workflows', label: '工作流' },
  { id: 'roles', label: '角色' },
  { id: 'skills', label: '技能' },
  { id: 'knowledge', label: '知识库' },
] as const;

export function AssetsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get('tab') ?? 'workflows';

  return (
    <div className="h-full flex flex-col min-h-0">
      <div className="px-4 sm:px-6 pt-4 border-b border-border/50 shrink-0">
        <h1 className="text-xl font-semibold mb-3">资产库</h1>
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
        {tab === 'workflows' && <WorkflowsPage />}
        {tab === 'roles' && <RolesPage />}
        {tab === 'skills' && <SkillsPage />}
        {tab === 'knowledge' && <WisdomPage />}
      </div>
    </div>
  );
}
