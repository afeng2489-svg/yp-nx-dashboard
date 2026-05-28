import { useState } from 'react';
import { Loader2, Sparkles } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { TEAM_TEMPLATES, type TeamTemplateId } from '@/data/teamTemplates';
import { useTeamStore, type Team } from '@/stores/teamStore';
import { unwrapEnvelope } from '@/api/response';
import { showError, showSuccess } from '@/lib/toast';

/** 从预置模板创建团队 */
export function TeamTemplatePicker({ compact = false }: { compact?: boolean }) {
  const { setCurrentTeam, fetchTeams } = useTeamStore();
  const [creating, setCreating] = useState<TeamTemplateId | null>(null);

  const handleCreate = async (templateId: TeamTemplateId) => {
    setCreating(templateId);
    try {
      const tpl = TEAM_TEMPLATES.find((t) => t.id === templateId);
      const res = await fetch(`${API_BASE_URL}/api/v1/teams/from-template`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ template: templateId, name: tpl?.label }),
      });
      if (!res.ok) throw new Error(`创建失败 (${res.status})`);
      const data = unwrapEnvelope<{ team: Team; roles: unknown[] }>(await res.json());
      setCurrentTeam(data.team);
      await fetchTeams();
      showSuccess(`已创建「${data.team.name}」`);
    } catch (e) {
      showError(e instanceof Error ? e.message : '创建失败');
    } finally {
      setCreating(null);
    }
  };

  return (
    <div className={compact ? 'flex flex-wrap gap-1.5' : 'grid sm:grid-cols-2 gap-3'}>
      {TEAM_TEMPLATES.map((tpl) => (
        <button
          key={tpl.id}
          type="button"
          disabled={creating !== null}
          onClick={() => void handleCreate(tpl.id)}
          className={
            compact
              ? 'text-[10px] px-2 py-1 rounded-md border border-border hover:bg-accent transition-colors flex items-center gap-1'
              : 'text-left p-3 rounded-xl border border-border/60 hover:border-primary/30 hover:bg-accent/40 transition-colors'
          }
        >
          {creating === tpl.id ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin shrink-0" />
          ) : compact ? null : (
            <Sparkles className="w-4 h-4 text-amber-500 shrink-0" />
          )}
          <span className={compact ? '' : 'block'}>
            <span className="font-medium text-sm">{tpl.label}</span>
            {!compact && (
              <span className="block text-xs text-muted-foreground mt-0.5">{tpl.description}</span>
            )}
          </span>
        </button>
      ))}
    </div>
  );
}
