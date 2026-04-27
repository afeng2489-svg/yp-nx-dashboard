import { useState, useEffect } from 'react';
import { X, Bot, Loader2, Plus, List } from 'lucide-react';
import { Role } from '@/stores/teamStore';
import { useTeamStore } from '@/stores/teamStore';
import { useAIConfigStore } from '@/stores/aiConfigStore';
import { useSkillStore } from '@/stores/skillStore';
import { showError } from '@/lib/toast';
import { SkillAssigner } from './SkillAssigner';

interface RoleEditorProps {
  role: Role | null;
  teamId: string;
  onClose: () => void;
  onSave: () => void;
}

export function RoleEditor({ role, teamId, onClose, onSave }: RoleEditorProps) {
  const [name, setName] = useState(role?.name || '');
  const [description, setDescription] = useState(role?.description || '');
  const [instructions, setInstructions] = useState(role?.instructions || '');
  const [model, setModel] = useState(role?.model || 'claude-sonnet-4-6');
  const [temperature, setTemperature] = useState(role?.temperature ?? 0.7);
  const [skillIds, setSkillIds] = useState<string[]>(role?.skills || []);
  const [showSkillAssigner, setShowSkillAssigner] = useState(false);
  const [saving, setSaving] = useState(false);

  const { createRole, updateRole } = useTeamStore();
  const { models, fetchModels } = useAIConfigStore();
  const { skills: skillList } = useSkillStore();

  // Fetch models on mount if not loaded
  useEffect(() => {
    if (models.length === 0) {
      fetchModels();
    }
  }, [models.length, fetchModels]);

  // Get skill names from IDs
  const selectedSkillNames = skillIds.map((id) => skillList.find((s) => s.id === id)?.name || id);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setSaving(true);
    try {
      const roleData = {
        name: name.trim(),
        description: description.trim() || '',
        system_prompt: instructions.trim() || '',
        model_config: {
          model_id: model,
          provider: 'anthropic',
          max_tokens: 4096,
          temperature,
          stop_sequences: [],
          extra_params: {},
        },
      };

      if (role) {
        await updateRole(role.id, roleData);
      } else {
        await createRole({ ...roleData, team_id: teamId });
      }
      onSave();
    } catch (error) {
      console.error('Failed to save role:', error);
      showError('操作失败', '保存角色失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
        <div className="relative w-full max-w-lg max-h-[90vh] bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-border/50">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25">
                <Bot className="w-5 h-5 text-white" />
              </div>
              <h2 className="text-lg font-semibold">{role ? '编辑角色' : '新建角色'}</h2>
            </div>
            <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">角色名称</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="输入角色名称"
                className="input-field"
                autoFocus
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">描述 (可选)</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="输入角色描述"
                className="input-field resize-none"
                rows={2}
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">指令</label>
              <textarea
                value={instructions}
                onChange={(e) => setInstructions(e.target.value)}
                placeholder="输入角色的系统指令..."
                className="input-field resize-none"
                rows={4}
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-2">模型</label>
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="input-field"
                >
                  {models.length > 0 ? (
                    models.map((m) => (
                      <option key={m.model_id} value={m.model_id}>
                        {m.display_name} ({m.model_id})
                      </option>
                    ))
                  ) : (
                    <option value="claude-sonnet-4-6">claude-sonnet-4-6</option>
                  )}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">
                  Temperature: {temperature.toFixed(1)}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  value={temperature}
                  onChange={(e) => setTemperature(parseFloat(e.target.value))}
                  className="w-full"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">技能</label>
              <button
                type="button"
                onClick={() => setShowSkillAssigner(true)}
                className="btn-secondary w-full justify-center"
              >
                <List className="w-4 h-4 mr-2" />
                选择技能 ({skillIds.length} 已选)
              </button>
              {selectedSkillNames.length > 0 && (
                <div className="flex flex-wrap gap-2 mt-2">
                  {selectedSkillNames.map((skillName, idx) => (
                    <span
                      key={skillIds[idx]}
                      className="inline-flex items-center gap-1 px-2.5 py-1 bg-muted rounded-lg text-sm"
                    >
                      {skillName}
                      <button
                        type="button"
                        onClick={() =>
                          setSkillIds((prev) => prev.filter((id) => id !== skillIds[idx]))
                        }
                        className="hover:text-red-500"
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </span>
                  ))}
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 pt-4 border-t border-border/50">
              <button type="button" onClick={onClose} className="btn-secondary">
                取消
              </button>
              <button type="submit" disabled={saving || !name.trim()} className="btn-primary">
                {saving ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    保存中...
                  </>
                ) : (
                  <>保存</>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>

      {/* Skill Assigner Modal */}
      {showSkillAssigner && (
        <SkillAssigner
          roleId={role?.id || ''}
          currentSkills={skillIds}
          onClose={() => setShowSkillAssigner(false)}
          onSkillsChange={(ids) => setSkillIds(ids)}
        />
      )}
    </>
  );
}
