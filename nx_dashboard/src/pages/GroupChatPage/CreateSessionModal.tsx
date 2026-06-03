import { MessagesSquare } from 'lucide-react';
import { CreateGroupSessionRequest, SpeakingStrategy } from '@/stores/groupChatStore';
import { Team } from '@/stores/teamStore';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import {
  FormField,
  FormSection,
  formControlClass,
} from '@/components/ui/formStyles';
import { cn } from '@/lib/utils';

const STRATEGY_LABELS: Record<SpeakingStrategy, string> = {
  round_robin: '轮流发言',
  free: '自由发言',
  moderator: '主持人模式',
  debate: '辩论模式',
};

export interface CreateSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  teams: Team[];
  createForm: CreateGroupSessionRequest;
  onFormChange: (form: CreateGroupSessionRequest) => void;
  onSubmit: () => void;
}

export function CreateSessionModal({
  isOpen,
  onClose,
  teams,
  createForm,
  onFormChange,
  onSubmit,
}: CreateSessionModalProps) {
  if (!isOpen) return null;

  const canSubmit =
    createForm.team_id &&
    createForm.team_id !== '__select__' &&
    createForm.name.trim() &&
    createForm.topic.trim();

  return (
    <LaunchModalShell
      onClose={onClose}
      title="新建讨论会话"
      subtitle="配置团队与讨论规则，创建后可邀请 Agent 参与"
      icon={<MessagesSquare />}
      accent="indigo"
      size="md"
      footer={
        <LaunchModalFooter
          onCancel={onClose}
          onSubmit={onSubmit}
          submitLabel="创建会话"
          disabled={!canSubmit}
          hint="标有 * 的字段为必填"
        />
      }
    >
      <div className="space-y-8">
        <FormSection title="会话信息">
          <FormField label="团队" required hint="选择参与讨论的团队">
            <Select
              value={createForm.team_id}
              onValueChange={(v) => onFormChange({ ...createForm, team_id: v })}
            >
              <SelectTrigger className={cn(formControlClass, 'cursor-pointer')}>
                <SelectValue placeholder="选择团队" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__select__">请选择…</SelectItem>
                {teams.map((team) => (
                  <SelectItem key={team.id} value={team.id}>
                    {team.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </FormField>

          <FormField label="会话名称" required hint="便于在列表中识别">
            <input
              type="text"
              value={createForm.name}
              onChange={(e) => onFormChange({ ...createForm, name: e.target.value })}
              className={formControlClass}
              placeholder="架构方案讨论"
            />
          </FormField>

          <FormField label="讨论主题" required hint="Agent 将围绕此主题展开讨论">
            <input
              type="text"
              value={createForm.topic}
              onChange={(e) => onFormChange({ ...createForm, topic: e.target.value })}
              className={formControlClass}
              placeholder="微服务 vs 单体架构"
            />
          </FormField>
        </FormSection>

        <FormSection title="讨论规则">
          <div className="grid gap-4 sm:grid-cols-2">
            <FormField label="发言策略">
              <Select
                value={createForm.speaking_strategy}
                onValueChange={(v) =>
                  onFormChange({ ...createForm, speaking_strategy: v as SpeakingStrategy })
                }
              >
                <SelectTrigger className={cn(formControlClass, 'cursor-pointer')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(Object.entries(STRATEGY_LABELS) as [SpeakingStrategy, string][]).map(
                    ([value, label]) => (
                      <SelectItem key={value} value={value}>
                        {label}
                      </SelectItem>
                    ),
                  )}
                </SelectContent>
              </Select>
            </FormField>

            <FormField label="最大回合" hint="达到上限后自动结束">
              <input
                type="number"
                value={createForm.max_turns}
                onChange={(e) =>
                  onFormChange({ ...createForm, max_turns: parseInt(e.target.value, 10) || 10 })
                }
                className={formControlClass}
                min={1}
                max={100}
              />
            </FormField>
          </div>
        </FormSection>
      </div>
    </LaunchModalShell>
  );
}
