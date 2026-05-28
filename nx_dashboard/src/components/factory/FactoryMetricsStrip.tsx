import { useEffect, useState } from 'react';
import { fetchFactoryMetrics, type FactoryMetrics } from '@/services/factoryMetrics';

/** Console 底部：6 项本地指标摘要 */
export function FactoryMetricsStrip() {
  const [metrics, setMetrics] = useState<FactoryMetrics | null>(null);

  useEffect(() => {
    void fetchFactoryMetrics().then(setMetrics);
    const t = setInterval(() => void fetchFactoryMetrics().then(setMetrics), 30_000);
    return () => clearInterval(t);
  }, []);

  if (!metrics) return null;

  const pct = (r: number) => `${Math.round(r * 100)}%`;

  return (
    <div className="rounded-xl border border-border/40 bg-muted/30 px-4 py-3 text-[11px] text-muted-foreground">
      <p className="font-medium text-foreground/80 mb-2">本地指标（AF-04）</p>
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
        <MetricCell label="激活" value={pct(metrics.activation.rate)} sub={`${metrics.activation.numerator}/${metrics.activation.denominator}`} />
        <MetricCell label="Golden Path" value={pct(metrics.golden_path_success.rate)} sub={`${metrics.golden_path_success.numerator}/${metrics.golden_path_success.denominator}`} />
        <MetricCell label="首 diff" value={`${metrics.time_to_first_diff.median_minutes.toFixed(1)}m`} sub={`n=${metrics.time_to_first_diff.samples}`} />
        <MetricCell label="Run 完成" value={pct(metrics.run_completion.rate)} sub={`${metrics.run_completion.numerator}/${metrics.run_completion.denominator}`} />
        <MetricCell label="终端降级" value={pct(metrics.terminal_fallback.rate)} sub={`${metrics.terminal_fallback.numerator}/${metrics.terminal_fallback.denominator}`} />
        <MetricCell label="W2 留存" value={pct(metrics.w2_retention.rate)} sub={`${metrics.w2_retention.numerator}/${metrics.w2_retention.denominator}`} />
      </div>
    </div>
  );
}

function MetricCell({
  label,
  value,
  sub,
}: {
  label: string;
  value: string;
  sub: string;
}) {
  return (
    <div>
      <div className="text-foreground/70">{label}</div>
      <div className="font-semibold text-foreground">{value}</div>
      <div>{sub}</div>
    </div>
  );
}
