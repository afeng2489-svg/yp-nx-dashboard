import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Calendar, Loader2 } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { fetchSpendSnapshot } from '@/services/costApi';

interface SprintRow {
  id: string;
  status: string;
}

function unwrap<T>(data: unknown): T {
  if (data && typeof data === 'object' && 'data' in data) {
    return (data as { data: T }).data;
  }
  return data as T;
}

/** Console Sprint 快照：进行中计数 + 本周成本 */
export function SprintSnapshotStrip() {
  const [inProgress, setInProgress] = useState<number | null>(null);
  const [weekUsd, setWeekUsd] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [sprintRes, spend] = await Promise.all([
          fetch(`${API_BASE_URL}/api/v1/sprints`),
          fetchSpendSnapshot().catch(() => ({ todayUsd: 0, weekUsd: 0 })),
        ]);
        if (cancelled) return;
        if (sprintRes.ok) {
          const rows = unwrap<SprintRow[]>(await sprintRes.json());
          const list = Array.isArray(rows) ? rows : [];
          setInProgress(list.filter((s) => s.status === 'in_progress').length);
        } else {
          setInProgress(0);
        }
        setWeekUsd(spend.weekUsd);
      } catch {
        if (!cancelled) {
          setInProgress(0);
          setWeekUsd(0);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground mt-3">
        <Loader2 className="w-3.5 h-3.5 animate-spin" />
        加载 Sprint 快照…
      </div>
    );
  }

  return (
    <div className="mt-3 flex flex-wrap items-center gap-3 rounded-xl border border-border/40 bg-muted/20 px-4 py-2.5 text-xs">
      <span className="inline-flex items-center gap-1.5 font-medium text-foreground/80">
        <Calendar className="w-3.5 h-3.5 text-primary" />
        Sprint 快照
      </span>
      <span className="text-muted-foreground">
        进行中 <strong className="text-foreground">{inProgress ?? 0}</strong> 项
      </span>
      <span className="text-muted-foreground">
        本周成本 <strong className="text-foreground">${(weekUsd ?? 0).toFixed(2)}</strong>
      </span>
      <Link to="/ops?tab=sprint" className="ml-auto text-primary hover:underline">
        打开 Sprint 板 →
      </Link>
    </div>
  );
}
