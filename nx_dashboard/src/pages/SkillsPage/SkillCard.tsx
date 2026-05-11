import { CheckSquare, Square } from 'lucide-react';
import type { SkillCardProps } from './types';

export function SkillCard({
  skill,
  multiSelectMode,
  isSelected,
  selectedSkillId,
  onSkillClick,
  onToggleSelect,
}: SkillCardProps) {
  const isDisabled = multiSelectMode && skill.is_preset;

  const handleClick = () => {
    if (multiSelectMode) {
      if (!skill.is_preset) onToggleSelect(skill.id);
    } else {
      onSkillClick(skill);
    }
  };

  const rowCls = [
    'w-full p-4 text-left transition-colors cursor-pointer',
    isDisabled ? 'opacity-50 cursor-not-allowed' : 'hover:bg-accent',
    !multiSelectMode && selectedSkillId === skill.id ? 'bg-primary/5' : '',
    multiSelectMode && isSelected ? 'bg-primary/10' : '',
  ].join(' ');

  return (
    <div key={skill.id} onClick={handleClick} className={rowCls}>
      <div className="flex items-start gap-3">
        {multiSelectMode && (
          <div
            className="pt-0.5"
            onClick={(e) => {
              e.stopPropagation();
              if (!skill.is_preset) onToggleSelect(skill.id);
            }}
          >
            {skill.is_preset ? (
              <Square className="w-4 h-4 text-muted-foreground/40" />
            ) : isSelected ? (
              <CheckSquare className="w-4 h-4 text-primary" />
            ) : (
              <Square className="w-4 h-4 text-muted-foreground" />
            )}
          </div>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between">
            <div className="font-medium text-foreground truncate">{skill.name}</div>
            {skill.is_preset && (
              <span className="px-2 py-0.5 text-xs bg-purple-500/10 text-purple-500 rounded shrink-0 ml-2">
                预设
              </span>
            )}
          </div>
          <div className="text-sm text-muted-foreground mt-1 line-clamp-2">{skill.description}</div>
          <div className="flex items-center gap-2 mt-2 flex-wrap">
            <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
              {skill.category.replace('_', ' ')}
            </span>
            {skill.tags.slice(0, 2).map((tag) => (
              <span key={tag} className="px-2 py-0.5 text-xs bg-primary/10 text-primary rounded">
                {tag}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
