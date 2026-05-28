import { GroupSession } from '@/stores/groupChatStore';
import { Role } from '@/stores/teamStore';
import {
  DISCUSSION_SCENE_PRESETS,
  pickRolesForPreset,
  type DiscussionScenePreset,
} from '@/data/discussionScenePresets';
import { cn } from '@/lib/utils';

export interface DiscussionSetupSheetProps {
  isOpen: boolean;
  onClose: () => void;
  currentSession: GroupSession;
  roles: Record<string, Role[]>;
  startForm: { participant_role_ids: string[] };
  onFormChange: (form: { participant_role_ids: string[] }) => void;
  onSubmit: () => void;
}

/** AF-UX-04b：讨论一步 Setup + 场景预设 */
export function DiscussionSetupSheet({
  isOpen,
  onClose,
  currentSession,
  roles,
  startForm,
  onFormChange,
  onSubmit,
}: DiscussionSetupSheetProps) {
  if (!isOpen) return null;

  const teamRoles = roles[currentSession.team_id] ?? [];

  const applyPreset = (preset: DiscussionScenePreset) => {
    onFormChange({ participant_role_ids: pickRolesForPreset(preset, teamRoles) });
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4" data-testid="discussion-setup-sheet">
      <div className="bg-card rounded-lg border w-full max-w-lg max-h-[90vh] overflow-y-auto p-6">
        <h2 className="text-xl font-bold mb-1">开始讨论</h2>
        <p className="text-sm text-muted-foreground mb-4">选场景 → 确认参与者 → 启动</p>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-4">
          {DISCUSSION_SCENE_PRESETS.map((preset) => (
            <button
              key={preset.id}
              type="button"
              className="text-left rounded-lg border border-border p-3 hover:border-primary/50 hover:bg-muted/30 transition-colors"
              data-testid={`discussion-scene-${preset.id}`}
              onClick={() => applyPreset(preset)}
            >
              <p className="text-sm font-medium">{preset.title}</p>
              <p className="text-[11px] text-muted-foreground mt-1 line-clamp-2">{preset.description}</p>
            </button>
          ))}
        </div>

        <p className="text-xs font-medium text-muted-foreground mb-2">参与角色</p>
        <div className="space-y-2 max-h-[220px] overflow-y-auto mb-4">
          {teamRoles.map((role) => (
            <label
              key={role.id}
              className={cn(
                'flex items-center gap-3 p-3 rounded-lg border hover:bg-accent cursor-pointer',
                startForm.participant_role_ids.includes(role.id) && 'border-primary/40 bg-primary/5',
              )}
            >
              <input
                type="checkbox"
                checked={startForm.participant_role_ids.includes(role.id)}
                onChange={(e) => {
                  const ids = e.target.checked
                    ? [...startForm.participant_role_ids, role.id]
                    : startForm.participant_role_ids.filter((id) => id !== role.id);
                  onFormChange({ participant_role_ids: ids });
                }}
              />
              <span className="text-sm font-medium">{role.name}</span>
            </label>
          ))}
        </div>

        <div className="flex justify-end gap-2">
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={startForm.participant_role_ids.length === 0}
            onClick={onSubmit}
          >
            启动讨论
          </button>
        </div>
      </div>
    </div>
  );
}
