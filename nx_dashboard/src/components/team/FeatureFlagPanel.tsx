import React from 'react';
import { useFeatureFlagStore, FlagState } from '../../stores/featureFlagStore';

const FLAG_LABELS: Record<string, string> = {
  pipeline: 'Pipeline 引擎',
  snapshot: '角色快照',
  crash_resume: '崩溃恢复',
  file_watch: '文件监控',
  process_lifecycle: '进程生命周期',
};

const STATE_OPTIONS: { value: FlagState; label: string; color: string }[] = [
  { value: 'on', label: '开启', color: 'bg-green-500' },
  { value: 'readonly', label: '只读', color: 'bg-yellow-500' },
  { value: 'off', label: '关闭', color: 'bg-gray-400' },
];

export default function FeatureFlagPanel() {
  const { flags, loading, error, fetchFlags, updateFlag, resetFlag } = useFeatureFlagStore();

  React.useEffect(() => {
    fetchFlags();
  }, [fetchFlags]);

  if (loading && flags.length === 0) {
    return <div className="text-sm text-gray-500 p-4">加载功能开关...</div>;
  }

  return (
    <div className="space-y-3 p-4">
      <h3 className="text-base font-bold">功能开关</h3>
      {error && <div className="text-sm text-red-500">{error}</div>}

      <div className="space-y-2">
        {flags.map((flag) => (
          <div
            key={flag.key}
            className="flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800"
          >
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium">{FLAG_LABELS[flag.key] || flag.key}</span>
              {flag.circuit_breaker && (
                <span className="px-2 py-0.5 rounded text-xs bg-red-100 text-red-700 font-medium">
                  熔断
                </span>
              )}
              {flag.error_count > 0 && (
                <span className="text-xs text-gray-400">
                  错误: {flag.error_count}/{flag.error_threshold}
                </span>
              )}
            </div>

            <div className="flex items-center gap-2">
              {/* Three-state toggle */}
              <div className="flex rounded-lg overflow-hidden border border-gray-300 dark:border-gray-600">
                {STATE_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    className={`px-2 py-1 text-xs transition-colors ${
                      flag.state === opt.value
                        ? `${opt.color} text-white`
                        : 'bg-gray-100 dark:bg-gray-700 text-gray-500 hover:bg-gray-200'
                    }`}
                    onClick={() => updateFlag(flag.key, opt.value)}
                    disabled={flag.circuit_breaker}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>

              {/* Reset button (only if circuit breaker tripped or has errors) */}
              {(flag.circuit_breaker || flag.error_count > 0) && (
                <button
                  className="px-2 py-1 text-xs rounded bg-blue-100 text-blue-700 hover:bg-blue-200"
                  onClick={() => resetFlag(flag.key)}
                >
                  重置
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
