import { useState } from 'react';
import { AtSign, Loader2, Send } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTeamStore, type Role } from '@/stores/teamStore';
import { executeRoleTask } from '@/services/teamExecute';
import { cn } from '@/lib/utils';

export interface FactoryRoleAskPanelProps {
  teamId?: string;
  onReply?: (roleName: string, content: string) => void;
}

/** AF-UX-06：工厂内嵌 @ 团队角色 quick ask */
export function FactoryRoleAskPanel({ teamId, onReply }: FactoryRoleAskPanelProps) {
  const roles = useTeamStore((s) => (teamId ? s.roles[teamId] ?? [] : []));
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<Role | null>(null);
  const [question, setQuestion] = useState('');
  const [loading, setLoading] = useState(false);

  if (!teamId || roles.length === 0) return null;

  const submit = async () => {
    if (!selected || !question.trim()) return;
    setLoading(true);
    try {
      const result = await executeRoleTask(selected.id, question.trim());
      const content = result.ok
        ? result.output?.trim() || '（无输出）'
        : result.error ?? '角色执行失败';
      onReply?.(selected.name, content);
      setQuestion('');
      setOpen(false);
    } finally {
      setLoading(false);
    }
  };

  if (!open) {
    return (
      <button
        type="button"
        className="text-xs text-muted-foreground hover:text-foreground inline-flex items-center gap-1"
        data-testid="factory-at-team"
        onClick={() => setOpen(true)}
      >
        <AtSign className="w-3.5 h-3.5" />
        和团队商量
      </button>
    );
  }

  return (
    <div className="rounded-lg border border-border/50 bg-muted/20 p-3 space-y-2" data-testid="factory-role-ask">
      <div className="flex flex-wrap gap-1.5">
        {roles.map((r) => (
          <button
            key={r.id}
            type="button"
            className={cn(
              'text-xs px-2 py-1 rounded-full border',
              selected?.id === r.id
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-border hover:border-primary/40',
            )}
            onClick={() => setSelected(r)}
          >
            @{r.name}
          </button>
        ))}
      </div>
      <div className="flex gap-2">
        <input
          className="flex-1 text-xs rounded-md border border-border px-2 py-1.5 bg-background"
          placeholder={selected ? `问 ${selected.name}…` : '先选角色'}
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          disabled={!selected}
        />
        <Button type="button" size="sm" className="h-8 px-2" disabled={loading || !selected} onClick={() => void submit()}>
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Send className="h-3.5 w-3.5" />}
        </Button>
      </div>
      <button type="button" className="text-[10px] text-muted-foreground hover:underline" onClick={() => setOpen(false)}>
        收起
      </button>
    </div>
  );
}
