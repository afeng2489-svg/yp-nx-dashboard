import React from 'react';
import { API_BASE_URL } from '../../api/constants';

interface ProcessStats {
  active_processes: number;
  max_processes: number;
  total_memory_bytes: number;
  max_memory_bytes: number;
  idle_candidates: string[];
  hibernated_candidates: string[];
}

export default function ProcessResourceBar() {
  const [stats, setStats] = React.useState<ProcessStats | null>(null);
  const [, setError] = React.useState(false);

  const fetchStats = React.useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/processes/stats`);
      if (res.ok) {
        const data = await res.json();
        setStats(data);
        setError(false);
      } else {
        setError(true);
      }
    } catch {
      setError(true);
    }
  }, []);

  React.useEffect(() => {
    fetchStats();
    const interval = setInterval(fetchStats, 10000); // 10s refresh
    return () => clearInterval(interval);
  }, [fetchStats]);

  if (!stats) return null;

  const memMB = Math.round(stats.total_memory_bytes / 1024 / 1024);
  const maxMemMB = Math.round(stats.max_memory_bytes / 1024 / 1024);
  const procPct =
    stats.max_processes > 0 ? (stats.active_processes / stats.max_processes) * 100 : 0;
  const memPct = maxMemMB > 0 ? (memMB / maxMemMB) * 100 : 0;

  return (
    <div className="flex items-center gap-3 text-xs">
      {/* Process count */}
      <div className="flex items-center gap-1.5">
        <span className="text-gray-500">进程</span>
        <div className="w-12 bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
          <div
            className={`h-1.5 rounded-full transition-all ${
              procPct > 80 ? 'bg-red-500' : procPct > 50 ? 'bg-yellow-500' : 'bg-green-500'
            }`}
            style={{ width: `${Math.min(procPct, 100)}%` }}
          />
        </div>
        <span className="text-gray-600 dark:text-gray-400">
          {stats.active_processes}/{stats.max_processes}
        </span>
      </div>

      {/* Memory */}
      <div className="flex items-center gap-1.5">
        <span className="text-gray-500">内存</span>
        <div className="w-14 bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
          <div
            className={`h-1.5 rounded-full transition-all ${
              memPct > 80 ? 'bg-red-500' : memPct > 50 ? 'bg-yellow-500' : 'bg-green-500'
            }`}
            style={{ width: `${Math.min(memPct, 100)}%` }}
          />
        </div>
        <span className="text-gray-600 dark:text-gray-400">
          {memMB}/{maxMemMB}MB
        </span>
      </div>

      {/* Warnings */}
      {stats.idle_candidates.length > 0 && (
        <span className="px-1.5 py-0.5 rounded bg-yellow-100 text-yellow-700 text-xs">
          {stats.idle_candidates.length} 闲置
        </span>
      )}
      {stats.hibernated_candidates.length > 0 && (
        <span className="px-1.5 py-0.5 rounded bg-red-100 text-red-600 text-xs">
          {stats.hibernated_candidates.length} 待回收
        </span>
      )}
    </div>
  );
}
