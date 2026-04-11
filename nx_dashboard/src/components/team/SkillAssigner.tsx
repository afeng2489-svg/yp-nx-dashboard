import { useState, useEffect } from 'react';
import { X, Plus, Search, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore } from '@/stores/teamStore';
import { useSkillStore } from '@/stores/skillStore';

interface SkillAssignerProps {
  roleId: string;
  currentSkills: string[];
  onClose: () => void;
  onSave?: () => void;
  onSkillsChange?: (skillIds: string[]) => void;
}

export function SkillAssigner({ roleId, currentSkills, onClose, onSave, onSkillsChange }: SkillAssignerProps) {
  const { assignSkill, removeSkill } = useTeamStore();
  const { skills: availableSkills, fetchSkills, loading } = useSkillStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSkillIds, setSelectedSkillIds] = useState<Set<string>>(new Set(currentSkills));

  useEffect(() => {
    fetchSkills();
  }, [fetchSkills]);

  const filteredSkills = availableSkills.filter(skill =>
    skill.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    skill.description?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleToggleSkill = async (skillId: string) => {
    let newIds: Set<string>;
    if (selectedSkillIds.has(skillId)) {
      newIds = new Set(selectedSkillIds);
      newIds.delete(skillId);
      setSelectedSkillIds(newIds);
      // Only call API if role already exists
      if (roleId) {
        await removeSkill(roleId, skillId);
      }
    } else {
      newIds = new Set(selectedSkillIds).add(skillId);
      setSelectedSkillIds(newIds);
      // Only call API if role already exists
      if (roleId) {
        await assignSkill(roleId, skillId);
      }
    }
    onSave?.();
    onSkillsChange?.(Array.from(newIds));
  };

  if (loading && availableSkills.length === 0) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
        <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden p-8">
          <div className="flex flex-col items-center justify-center">
            <Loader2 className="w-8 h-8 animate-spin text-primary mb-2" />
            <p className="text-muted-foreground">加载技能列表...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50">
          <h2 className="text-lg font-semibold">分配技能</h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Search */}
        <div className="px-6 py-4 border-b border-border/50">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="搜索技能..."
              className="w-full pl-10 pr-4 py-2 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
          </div>
        </div>

        {/* Skills List */}
        <div className="max-h-64 overflow-y-auto p-4 space-y-2">
          {filteredSkills.length === 0 ? (
            <div className="text-center py-4 text-muted-foreground">
              <p className="text-sm">没有找到技能</p>
            </div>
          ) : (
            filteredSkills.map((skill) => {
              const isSelected = selectedSkillIds.has(skill.id);
              return (
                <button
                  key={skill.id}
                  onClick={() => handleToggleSkill(skill.id)}
                  className={cn(
                    'w-full text-left p-3 rounded-lg border transition-all',
                    isSelected
                      ? 'bg-primary/5 border-primary/30'
                      : 'bg-card border-border/50 hover:border-primary/20'
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="font-medium">{skill.name}</p>
                      <p className="text-xs text-muted-foreground">{skill.description}</p>
                    </div>
                    <div
                      className={cn(
                        'w-5 h-5 rounded border flex items-center justify-center',
                        isSelected ? 'bg-primary border-primary text-white' : 'border-muted'
                      )}
                    >
                      {isSelected && <Plus className="w-3 h-3" />}
                    </div>
                  </div>
                </button>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-border/50 flex justify-between items-center">
          <p className="text-sm text-muted-foreground">
            已选择 {selectedSkillIds.size} 个技能
          </p>
          <button onClick={onClose} className="btn-primary">
            完成
          </button>
        </div>
      </div>
    </div>
  );
}
