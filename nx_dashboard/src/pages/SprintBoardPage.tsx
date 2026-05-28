import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { API_BASE_URL } from '@/api/constants';
import { cn } from '@/lib/utils';
import { Trash2, RefreshCw, Clock, AlertCircle, List, Play } from 'lucide-react';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { Pagination } from '@/components/ui/Pagination';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageEmptyState } from '@/components/ui/PageEmptyState';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

interface SprintCard {
  id: string;
  title: string;
  status: string;
  priority: string;
  estimated_hours: number;
  data_json: string;
  updated_at: string;
}

const STATUS_TABS = [
  { key: 'all', label: '全部' },
  { key: 'pending', label: '待开始' },
  { key: 'in_progress', label: '进行中' },
  { key: 'completed', label: '已完成' },
  { key: 'skipped', label: '跳过' },
  { key: 'blocked', label: '阻塞' },
] as const;

const STATUS_COLORS: Record<string, string> = {
  completed: 'bg-green-500/20 text-green-400 border-green-500/30',
  in_progress: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
  pending: 'bg-zinc-500/20 text-zinc-400 border-zinc-500/30',
  skipped: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  blocked: 'bg-red-500/20 text-red-400 border-red-500/30',
};

const STATUS_LABELS: Record<string, string> = {
  pending: '待开始',
  in_progress: '进行中',
  completed: '已完成',
  skipped: '跳过',
  blocked: '阻塞',
};

const PRIORITY_COLORS: Record<string, string> = {
  P0: 'bg-red-500/20 text-red-400 border-red-500/30',
  P1: 'bg-orange-500/20 text-orange-400 border-orange-500/30',
  P2: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
  P3: 'bg-zinc-500/20 text-zinc-400 border-zinc-500/30',
};

const PAGE_SIZE = 15;

async function fetchSprints(): Promise<SprintCard[]> {
  const res = await fetch(`${API_BASE_URL}/api/v1/sprints`);
  if (!res.ok) throw new Error('fetch failed');
  return res.json();
}

async function patchStatus(id: string, status: string) {
  const res = await fetch(`${API_BASE_URL}/api/v1/sprints/${id}/status`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status }),
  });
  if (!res.ok) throw new Error('patch status failed');
}

async function deleteSprint(id: string) {
  const res = await fetch(`${API_BASE_URL}/api/v1/sprints/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error('delete failed');
}

export function SprintBoardPage({ embedded = false }: { embedded?: boolean }) {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { data: sprints = [], isLoading } = useQuery({
    queryKey: ['sprints'],
    queryFn: fetchSprints,
    staleTime: 0,
  });

  const statusMutation = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) => patchStatus(id, status),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['sprints'] }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteSprint(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['sprints'] }),
  });

  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [page, setPage] = useState(1);
  const [confirmDelete, setConfirmDelete] = useState<SprintCard | null>(null);

  const statusCounts = STATUS_TABS.reduce(
    (acc, { key }) => {
      acc[key] = key === 'all' ? sprints.length : sprints.filter((s) => s.status === key).length;
      return acc;
    },
    {} as Record<string, number>,
  );

  const filtered =
    statusFilter === 'all' ? sprints : sprints.filter((s) => s.status === statusFilter);

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const paged = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  const handleFilterChange = (key: string) => {
    setStatusFilter(key);
    setPage(1);
  };

  const launchInFactory = (card: SprintCard) => {
    navigate(
      `/factory?intent=${encodeURIComponent(card.title)}&sprint_id=${encodeURIComponent(card.id)}`,
    );
  };

  return (
    <div className={embedded ? 'space-y-4 p-4' : 'page-container space-y-6'}>
      {!embedded && (
        <PageHeader
          title="Sprint 看板"
          description={`共 ${sprints.length} 个 Sprint`}
          actions={
            <button
              onClick={() => qc.invalidateQueries({ queryKey: ['sprints'] })}
              className="btn-secondary flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              刷新
            </button>
          }
        />
      )}

      <div className="flex items-center gap-1 bg-accent/50 rounded-xl p-1 w-fit">
        {STATUS_TABS.map(({ key, label }) => (
          <button
            key={key}
            onClick={() => handleFilterChange(key)}
            className={cn(
              'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
              statusFilter === key
                ? 'bg-card shadow-sm text-foreground'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {label}
            <span
              className={cn(
                'px-1.5 py-0.5 rounded-full text-[11px] font-medium',
                statusFilter === key
                  ? 'bg-indigo-500/10 text-indigo-600'
                  : 'bg-muted text-muted-foreground',
              )}
            >
              {statusCounts[key]}
            </span>
          </button>
        ))}
      </div>

      {isLoading && sprints.length === 0 && (
        <div className="animate-pulse space-y-3">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="h-14 bg-muted rounded-xl" />
          ))}
        </div>
      )}

      {filtered.length === 0 && !isLoading ? (
        <PageEmptyState
          icon={List}
          title="暂无 Sprint"
          description={
            statusFilter === 'all'
              ? '还没有创建任何 Sprint'
              : `没有「${STATUS_LABELS[statusFilter] ?? statusFilter}」状态的 Sprint`
          }
        />
      ) : (
        <>
          <div className="bg-card rounded-xl border border-border/50 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border/50 bg-accent/30">
                  <th className="text-left px-4 py-3 font-medium text-muted-foreground">标题</th>
                  <th className="text-left px-4 py-3 font-medium text-muted-foreground w-24">
                    优先级
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-muted-foreground w-28">
                    状态
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-muted-foreground w-24">
                    预估
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-muted-foreground w-36">
                    更新时间
                  </th>
                  <th className="text-right px-4 py-3 font-medium text-muted-foreground w-20">
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                {paged.map((card) => {
                  const data = (() => {
                    try {
                      return JSON.parse(card.data_json);
                    } catch {
                      return {};
                    }
                  })();

                  return (
                    <tr
                      key={card.id}
                      className="border-b border-border/30 last:border-0 hover:bg-accent/30 transition-colors group"
                    >
                      <td className="px-4 py-3">
                        <p className="font-medium truncate max-w-md">{card.title}</p>
                        {data.why && (
                          <p className="text-xs text-muted-foreground truncate max-w-md mt-0.5">
                            {data.why}
                          </p>
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={cn(
                            'px-2 py-0.5 rounded-full text-xs font-medium border',
                            PRIORITY_COLORS[card.priority] ?? '',
                          )}
                        >
                          {card.priority}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <Select
                          value={card.status}
                          onValueChange={(v) => statusMutation.mutate({ id: card.id, status: v })}
                        >
                          <SelectTrigger className="h-7 text-xs w-24">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="pending">待开始</SelectItem>
                            <SelectItem value="in_progress">进行中</SelectItem>
                            <SelectItem value="completed">已完成</SelectItem>
                            <SelectItem value="skipped">跳过</SelectItem>
                            <SelectItem value="blocked">阻塞</SelectItem>
                          </SelectContent>
                        </Select>
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">
                        {card.estimated_hours > 0 ? `${card.estimated_hours}h` : '-'}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-1.5 text-muted-foreground">
                          <Clock className="w-3.5 h-3.5" />
                          <span>
                            {new Date(card.updated_at).toLocaleString('zh-CN', {
                              month: 'numeric',
                              day: 'numeric',
                              hour: '2-digit',
                              minute: '2-digit',
                            })}
                          </span>
                        </div>
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="flex items-center justify-end gap-1">
                          {(card.status === 'pending' || card.status === 'in_progress') && (
                            <button
                              type="button"
                              onClick={() => launchInFactory(card)}
                              className="p-1.5 rounded-lg hover:bg-primary/10 text-primary transition-all"
                              title="AI 做 — 跳转工厂台"
                            >
                              <Play className="w-4 h-4" />
                            </button>
                          )}
                          <button
                            onClick={() => setConfirmDelete(card)}
                            className="p-1.5 rounded-lg hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-all opacity-0 group-hover:opacity-100"
                            title="删除"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
        </>
      )}

      <ConfirmModal
        isOpen={!!confirmDelete}
        title="删除 Sprint"
        message={`确定删除 "${confirmDelete?.title}"？此操作无法撤销。`}
        confirmText="删除"
        cancelText="取消"
        variant="danger"
        onConfirm={() => {
          if (confirmDelete) deleteMutation.mutate(confirmDelete.id);
          setConfirmDelete(null);
        }}
        onCancel={() => setConfirmDelete(null)}
      />
    </div>
  );
}
