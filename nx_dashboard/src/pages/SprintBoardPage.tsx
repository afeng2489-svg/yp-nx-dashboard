import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { API_BASE_URL } from '@/api/constants';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface SprintCard {
  id: string;
  title: string;
  status: string;
  priority: string;
  estimated_hours: number;
  data_json: string;
  updated_at: string;
}

const STATUS_COLORS: Record<string, string> = {
  completed: 'bg-green-500/20 text-green-400 border-green-500/30',
  in_progress: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
  pending: 'bg-zinc-500/20 text-zinc-400 border-zinc-500/30',
  skipped: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  blocked: 'bg-red-500/20 text-red-400 border-red-500/30',
};

const PRIORITY_COLORS: Record<string, string> = {
  P0: 'bg-red-500/20 text-red-400',
  P1: 'bg-orange-500/20 text-orange-400',
  P2: 'bg-blue-500/20 text-blue-400',
  P3: 'bg-zinc-500/20 text-zinc-400',
};

async function fetchSprints(): Promise<SprintCard[]> {
  const res = await fetch(`${API_BASE_URL}/api/v1/sprints`);
  if (!res.ok) throw new Error('fetch failed');
  return res.json();
}

async function patchStatus(id: string, status: string) {
  await fetch(`${API_BASE_URL}/api/v1/sprints/${id}/status`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status }),
  });
}

export function SprintBoardPage() {
  const qc = useQueryClient();
  const { data: sprints = [], isLoading } = useQuery({
    queryKey: ['sprints'],
    queryFn: fetchSprints,
    staleTime: 0,
  });

  const mutation = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) => patchStatus(id, status),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['sprints'] }),
  });

  if (isLoading) {
    return <div className="flex items-center justify-center h-64 text-muted-foreground">加载中...</div>;
  }

  const byStatus = (s: string) => sprints.filter((c) => c.status === s);
  const columns = [
    { key: 'pending', label: '待开始' },
    { key: 'in_progress', label: '进行中' },
    { key: 'completed', label: '已完成' },
    { key: 'skipped', label: '跳过' },
    { key: 'blocked', label: '阻塞' },
  ];

  return (
    <div className="p-6 space-y-4">
      <h1 className="text-xl font-bold">Sprint 看板</h1>
      <p className="text-sm text-muted-foreground">共 {sprints.length} 个 Sprint</p>

      <div className="grid grid-cols-5 gap-3">
        {columns.map(({ key, label }) => (
          <div key={key} className="space-y-2">
            <div className={cn('text-xs font-semibold px-2 py-1 rounded border', STATUS_COLORS[key])}>
              {label} ({byStatus(key).length})
            </div>
            {byStatus(key).map((card) => (
              <SprintCardItem
                key={card.id}
                card={card}
                onStatusChange={(status) => mutation.mutate({ id: card.id, status })}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function SprintCardItem({
  card,
  onStatusChange,
}: {
  card: SprintCard;
  onStatusChange: (s: string) => void;
}) {
  const data = (() => {
    try { return JSON.parse(card.data_json); } catch { return {}; }
  })();

  return (
    <div className="rounded-lg border border-border/50 bg-card p-3 space-y-2 text-sm">
      <div className="font-medium leading-snug">{card.title}</div>
      <div className="flex items-center gap-1.5 flex-wrap">
        <Badge className={cn('text-[10px] px-1.5 py-0', PRIORITY_COLORS[card.priority] ?? '')}>
          {card.priority}
        </Badge>
        {card.estimated_hours > 0 && (
          <span className="text-[10px] text-muted-foreground">{card.estimated_hours}h</span>
        )}
      </div>
      {data.why && (
        <p className="text-[11px] text-muted-foreground line-clamp-2">{data.why}</p>
      )}
      <select
        className="w-full text-[11px] bg-background border border-border rounded px-1 py-0.5"
        value={card.status}
        onChange={(e) => onStatusChange(e.target.value)}
      >
        <option value="pending">待开始</option>
        <option value="in_progress">进行中</option>
        <option value="completed">已完成</option>
        <option value="skipped">跳过</option>
        <option value="blocked">阻塞</option>
      </select>
    </div>
  );
}
