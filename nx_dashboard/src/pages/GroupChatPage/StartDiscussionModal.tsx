import { Users } from 'lucide-react';
import { GroupSession } from '@/stores/groupChatStore';
import { Role } from '@/stores/teamStore';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import { cn } from '@/lib/utils';

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

  const sessionRoles = roles[currentSession.team_id] || [];
  const count = startForm.participant_role_ids.length;

  return (
    <LaunchModalShell
      onClose={onClose}
      title="开始讨论"
      subtitle="勾选参与讨论的角色，至少选择一位"
      icon={<Users />}
      accent="indigo"
      size="md"
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={onSubmit}
          submitLabel="开始讨论"
          disabled={count === 0}
          hint={count > 0 ? `已选 ${count} 位参与者` : '请至少选择一位角色'}
        />
      }
    >
      <div className="max-h-[min(360px,45vh)] space-y-2 overflow-y-auto overscroll-contain pr-0.5">
        {sessionRoles.map((role) => {
          const checked = startForm.participant_role_ids.includes(role.id);
          return (
            <label
              key={role.id}
              className={cn(
                'flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-3 transition-colors',
                checked
                  ? 'border-primary/50 bg-primary/5'
                  : 'border-border hover:bg-muted/40',
              )}
            >
              <input
                type="checkbox"
                checked={checked}
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
                className="mt-1 rounded border-input text-primary focus:ring-primary/30"
              />
              <div className="min-w-0 flex-1">
                <span className="text-sm font-medium">{role.name}</span>
                {role.description && (
                  <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
                    {role.description}
                  </p>
                )}
              </div>
            </label>
          );
        })}
        {sessionRoles.length === 0 && (
          <p className="py-10 text-center text-sm text-muted-foreground">该团队暂无可用角色</p>
        )}
      </div>
    </LaunchModalShell>
  );
}
