import { CreateGroupSessionRequest, SpeakingStrategy } from '@/stores/groupChatStore';
import { Team } from '@/stores/teamStore';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

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

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
        <h2 className="text-xl font-bold mb-4">新建讨论会话</h2>
        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium mb-1 block">团队</label>
            <Select
              value={createForm.team_id}
              onValueChange={(v) => onFormChange({ ...createForm, team_id: v })}
            >
              <SelectTrigger className="h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="">选择团队</SelectItem>
                {teams.map((team) => (
                  <SelectItem key={team.id} value={team.id}>
                    {team.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div>
            <label className="text-sm font-medium mb-1 block">会话名称</label>
            <input
              type="text"
              value={createForm.name}
              onChange={(e) => onFormChange({ ...createForm, name: e.target.value })}
              className="input w-full"
              placeholder="架构方案讨论"
            />
          </div>
          <div>
            <label className="text-sm font-medium mb-1 block">讨论主题</label>
            <input
              type="text"
              value={createForm.topic}
              onChange={(e) => onFormChange({ ...createForm, topic: e.target.value })}
              className="input w-full"
              placeholder="微服务 vs 单体架构"
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-sm font-medium mb-1 block">发言策略</label>
              <Select
                value={createForm.speaking_strategy}
                onValueChange={(v) =>
                  onFormChange({ ...createForm, speaking_strategy: v as SpeakingStrategy })
                }
              >
                <SelectTrigger className="h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="round_robin">轮流发言</SelectItem>
                  <SelectItem value="free">自由发言</SelectItem>
                  <SelectItem value="moderator">主持人模式</SelectItem>
                  <SelectItem value="debate">辩论模式</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">最大回合</label>
              <input
                type="number"
                value={createForm.max_turns}
                onChange={(e) =>
                  onFormChange({ ...createForm, max_turns: parseInt(e.target.value) || 10 })
                }
                className="input w-full"
                min={1}
                max={100}
              />
            </div>
          </div>
        </div>
        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="btn btn-outline">
            取消
          </button>
          <button
            onClick={onSubmit}
            disabled={!createForm.team_id || !createForm.name || !createForm.topic}
            className="btn btn-primary"
          >
            创建
          </button>
        </div>
      </div>
    </div>
  );
}
