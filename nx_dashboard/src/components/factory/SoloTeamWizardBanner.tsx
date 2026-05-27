import { useState } from 'react';
import { Loader2, Sparkles, Users } from 'lucide-react';
import { useTeamStore } from '@/stores/teamStore';
import { showError, showSuccess } from '@/lib/toast';

/** Solo 团队 3 分钟向导 MVP（F5 — 待 from-template API） */
export function SoloTeamWizardBanner() {
  const { teams, currentTeam, createTeam, setCurrentTeam, fetchTeams } = useTeamStore();
  const [creating, setCreating] = useState(false);

  const hasSolo = teams.some((t) => /solo|全栈|一人/i.test(t.name));

  if (currentTeam || hasSolo) return null;

  const handleCreate = async () => {
    setCreating(true);
    try {
      const team = await createTeam({
        name: 'Solo 全栈',
        description: 'AF-01 默认一人团队 — 绑定 solo-dev Golden Path',
      });
      setCurrentTeam(team);
      await fetchTeams();
      showSuccess('Solo 团队已创建，可在顶栏切换');
    } catch (e) {
      showError(e instanceof Error ? e.message : '创建失败');
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="rounded-xl border border-indigo-500/30 bg-indigo-500/5 p-4 flex flex-col sm:flex-row sm:items-center gap-4">
      <div className="flex items-start gap-3 flex-1">
        <div className="p-2 rounded-lg bg-indigo-500/10">
          <Users className="w-5 h-5 text-indigo-600" />
        </div>
        <div>
          <p className="font-medium text-sm flex items-center gap-2">
            创建 Solo 团队
            <Sparkles className="w-3.5 h-3.5 text-amber-500" />
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            约 1 分钟：默认全栈角色 + solo-dev 工作流，无需打开终端
          </p>
        </div>
      </div>
      <button
        type="button"
        onClick={() => void handleCreate()}
        disabled={creating}
        className="btn-primary shrink-0 flex items-center gap-2"
      >
        {creating ? <Loader2 className="w-4 h-4 animate-spin" /> : null}
        一键创建
      </button>
    </div>
  );
}
