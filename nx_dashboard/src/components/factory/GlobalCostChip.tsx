import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { DollarSign } from 'lucide-react';
import { fetchSpendSnapshot } from '@/services/costApi';

/** 顶栏成本摘要 — 点击跳转运营成本 Tab */
export function GlobalCostChip() {
  const navigate = useNavigate();
  const [weekUsd, setWeekUsd] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchSpendSnapshot()
      .then(({ weekUsd: w }) => {
        if (!cancelled) setWeekUsd(w);
      })
      .catch(() => {
        if (!cancelled) setWeekUsd(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (weekUsd == null) return null;

  const label = weekUsd < 0.01 ? '$0' : `$${weekUsd.toFixed(2)}`;

  return (
    <button
      type="button"
      onClick={() => navigate('/ops?tab=cost')}
      className="hidden sm:inline-flex items-center gap-1 text-[10px] sm:text-xs px-2 py-0.5 rounded-full border border-border/60 bg-muted/30 hover:bg-accent transition-colors"
      title="近 7 日 Run 成本 · 点击查看详情"
    >
      <DollarSign className="w-3 h-3 opacity-70" />
      <span>{label}</span>
      <span className="text-muted-foreground hidden lg:inline">/ 7d</span>
    </button>
  );
}
