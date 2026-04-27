import { Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Team } from '@/stores/teamStore';

interface TeamListProps {
  teams: Team[];
  selectedTeamId?: string;
  onSelect: (team: Team) => void;
  onCreate: () => void;
}

export function TeamList({ teams, selectedTeamId, onSelect, onCreate }: TeamListProps) {
  if (teams.length === 0) {
    return (
      <div className="text-center py-8">
        <p className="text-muted-foreground mb-4">暂无团队</p>
        <button onClick={onCreate} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建团队
        </button>
      </div>
    );
  }

  return (
    <div className="grid gap-3">
      {teams.map((team) => (
        <button
          key={team.id}
          onClick={() => onSelect(team)}
          className={cn(
            'w-full text-left p-4 rounded-xl border transition-all duration-200',
            selectedTeamId === team.id
              ? 'bg-primary/5 border-primary/30'
              : 'bg-card border-border/50 hover:border-primary/20 hover:bg-primary/5',
          )}
        >
          <div className="flex items-center justify-between">
            <div>
              <h3 className="font-semibold">{team.name}</h3>
              <p className="text-sm text-muted-foreground line-clamp-1">
                {team.description || '无描述'}
              </p>
            </div>
            <span
              className={cn(
                'w-2 h-2 rounded-full',
                selectedTeamId === team.id ? 'bg-primary' : 'bg-muted',
              )}
            />
          </div>
        </button>
      ))}
    </div>
  );
}
