import { GroupSession } from '@/stores/groupChatStore';
import { Role } from '@/stores/teamStore';

export interface StartDiscussionModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentSession: GroupSession;
  roles: Record<string, Role[]>;
  startForm: { participant_role_ids: string[] };
  onFormChange: (form: { participant_role_ids: string[] }) => void;
  onSubmit: () => void;
}

export function StartDiscussionModal({
  isOpen,
  onClose,
  currentSession,
  roles,
  startForm,
  onFormChange,
  onSubmit,
}: StartDiscussionModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
        <h2 className="text-xl font-bold mb-4">开始讨论</h2>
        <p className="text-sm text-muted-foreground mb-4">
          选择参与讨论的角色（当前团队中的角色将作为讨论参与者）
        </p>
        <div className="space-y-2 max-h-[300px] overflow-y-auto">
          {(roles[currentSession.team_id] || []).map((role) => (
            <label
              key={role.id}
              className="flex items-center gap-3 p-3 rounded-lg border hover:bg-accent cursor-pointer"
            >
              <input
                type="checkbox"
                checked={startForm.participant_role_ids.includes(role.id)}
                onChange={(e) => {
                  if (e.target.checked) {
                    onFormChange({
                      participant_role_ids: [...startForm.participant_role_ids, role.id],
                    });
                  } else {
                    onFormChange({
                      participant_role_ids: startForm.participant_role_ids.filter(
                        (id) => id !== role.id,
                      ),
                    });
                  }
                }}
                className="rounded"
              />
              <div>
                <span className="font-medium">{role.name}</span>
                <p className="text-xs text-muted-foreground">{role.description}</p>
              </div>
            </label>
          ))}
        </div>
        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="btn btn-outline">
            取消
          </button>
          <button
            onClick={onSubmit}
            disabled={startForm.participant_role_ids.length === 0}
            className="btn btn-primary"
          >
            开始
          </button>
        </div>
      </div>
    </div>
  );
}
