import { useSearchParams } from 'react-router-dom';
import { WorkflowsPage } from '@/pages/WorkflowsPage';
import { RolesPage } from '@/pages/RolesPage';
import { SkillsPage } from '@/pages/SkillsPage';
import { WisdomPage } from '@/pages/WisdomPage';
import { KnowledgeBasePage } from '@/pages/KnowledgeBasePage';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageTabs } from '@/components/ui/PageTabs';

const TABS = [
  { id: 'workflows', label: '产线模板' },
  { id: 'skills', label: '技能' },
  { id: 'roles', label: '角色库' },
  { id: 'knowledge', label: '知识库' },
] as const;

const KNOWLEDGE_SUBTABS = [
  { id: 'rag', label: 'RAG 知识库' },
  { id: 'wisdom', label: '智慧库' },
] as const;

/** AF-07 / AF-10 / AF-11 资产库 — 四 Tab + 知识库子 Tab */
export function AssetsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get('tab') ?? 'workflows';
  const knowledgeSub = searchParams.get('sub') ?? 'rag';

  const setKnowledgeSub = (sub: string) => {
    setSearchParams({ tab: 'knowledge', sub });
  };

  return (
    <div className="h-full flex flex-col min-h-0">
      <div className="px-4 sm:px-6 pt-4 lg:px-8 lg:pt-6 border-b border-border/50 shrink-0 space-y-3">
        <PageHeader
          title="资产库"
          description="产线、技能、角色与知识 — 工厂 Run 的配置来源"
        />
        <PageTabs
          items={[...TABS]}
          value={tab}
          onValueChange={(id) => setSearchParams({ tab: id })}
        />
        {tab === 'knowledge' && (
          <div className="pb-2">
            <PageTabs
              items={[...KNOWLEDGE_SUBTABS]}
              value={knowledgeSub}
              onValueChange={setKnowledgeSub}
              variant="pills"
            />
          </div>
        )}
      </div>
      <div className="flex-1 overflow-auto min-h-0">
        {tab === 'workflows' && <WorkflowsPage embedded />}
        {tab === 'roles' && <RolesPage embedded />}
        {tab === 'skills' && <SkillsPage embedded />}
        {tab === 'knowledge' && knowledgeSub === 'wisdom' && <WisdomPage embedded />}
        {tab === 'knowledge' && knowledgeSub !== 'wisdom' && <KnowledgeBasePage embedded />}
      </div>
    </div>
  );
}
