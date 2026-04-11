import { Edit, Trash2, Bot, Code, Settings } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Role } from '@/stores/teamStore';

interface RoleCardProps {
  role: Role;
  onEdit: () => void;
  onDelete: () => void;
}

export function RoleCard({ role, onEdit, onDelete }: RoleCardProps) {
  return (
    <div className="bg-gradient-to-r from-purple-500/5 to-pink-500/5 rounded-xl p-4 border border-purple-500/10">
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Bot className="w-4 h-4 text-purple-500" />
            <span className="font-semibold">{role.name}</span>
            {role.model && (
              <span className="px-2 py-0.5 text-xs bg-gradient-to-r from-indigo-500/10 to-purple-500/10 rounded-full border border-indigo-500/20 text-indigo-600">
                {role.model}
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground/80 line-clamp-2 mb-2">
            {role.description || role.instructions || '暂无描述'}
          </p>
          {role.skills && role.skills.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {role.skills.slice(0, 3).map((skill) => (
                <span
                  key={skill}
                  className="px-2 py-0.5 text-xs bg-card rounded border border-border"
                >
                  {skill}
                </span>
              ))}
              {role.skills.length > 3 && (
                <span className="px-2 py-0.5 text-xs text-muted-foreground">
                  +{role.skills.length - 3} more
                </span>
              )}
            </div>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={onEdit}
            className={cn(
              'p-2 rounded-lg transition-all duration-200',
              'hover:bg-indigo-500/10 text-muted-foreground hover:text-indigo-500'
            )}
            title="编辑"
          >
            <Edit className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onDelete}
            className={cn(
              'p-2 rounded-lg transition-all duration-200',
              'hover:bg-red-500/10 text-muted-foreground hover:text-red-500'
            )}
            title="删除"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
