import React from 'react';
import { useSnapshotStore } from '../../stores/snapshotStore';

const PHASE_COLORS: Record<string, string> = {
  idle: 'bg-gray-400',
  thinking: 'bg-blue-400',
  coding: 'bg-green-500',
  testing: 'bg-orange-400',
  done: 'bg-green-600',
  failed: 'bg-red-500',
  hibernated: 'bg-gray-300',
};

const PHASE_LABELS: Record<string, string> = {
  idle: '空闲',
  thinking: '思考中',
  coding: '编码中',
  testing: '测试中',
  done: '已完成',
  failed: '失败',
  hibernated: '休眠',
};

interface Props {
  projectId: string;
}

export default function ProjectProgressDashboard({ projectId }: Props) {
  const { progress, snapshots, progressLoading, snapshotsLoading, error, fetchProgress, fetchSnapshots } = useSnapshotStore();

  React.useEffect(() => {
    if (projectId) {
      fetchProgress(projectId);
      fetchSnapshots(projectId);
    }
  }, [projectId, fetchProgress, fetchSnapshots]);

  if (!projectId) {
    return <div className="text-sm text-gray-400 p-4">请先打开一个项目工作区</div>;
  }

  const isLoading = (progressLoading || snapshotsLoading) && !progress && snapshots.length === 0;

  if (isLoading) {
    return <div className="text-sm text-gray-500 p-4">加载进度...</div>;
  }

  if (error) {
    return <div className="text-sm text-red-400 p-4">{error}</div>;
  }

  return (
    <div className="space-y-4 p-4">
      {/* Overall progress */}
      {progress && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h3 className="text-base font-bold">项目进度</h3>
            <span className="text-sm text-gray-500">
              {progress.overall_pct}%
            </span>
          </div>

          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
            <div
              className="bg-blue-500 h-3 rounded-full transition-all duration-500"
              style={{ width: `${progress.overall_pct}%` }}
            />
          </div>

          <div className="flex gap-4 text-xs text-gray-500">
            <span>阶段: {progress.overall_phase}</span>
            <span>总角色: {progress.total_roles}</span>
            <span className="text-green-600">活跃: {progress.active_roles}</span>
            <span className="text-green-700">完成: {progress.completed_roles}</span>
            {progress.failed_roles > 0 && (
              <span className="text-red-500">失败: {progress.failed_roles}</span>
            )}
          </div>

          {progress.last_activity && (
            <p className="text-xs text-gray-400 truncate">
              最近活动: {progress.last_activity}
            </p>
          )}
        </div>
      )}

      {/* Role list */}
      {snapshots.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-gray-600 dark:text-gray-400">角色进度</h4>
          {snapshots.map(snap => (
            <RoleProgressCard key={snap.id} snapshot={snap} />
          ))}
        </div>
      )}
    </div>
  );
}

function RoleProgressCard({ snapshot }: { snapshot: ReturnType<typeof useSnapshotStore.getState>['snapshots'][0] }) {
  const [expanded, setExpanded] = React.useState(false);
  const phaseColor = PHASE_COLORS[snapshot.phase] || 'bg-gray-400';
  const phaseLabel = PHASE_LABELS[snapshot.phase] || snapshot.phase;

  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 overflow-hidden">
      {/* Header */}
      <button
        className="w-full flex items-center gap-3 p-3 text-left hover:bg-gray-50 dark:hover:bg-gray-750"
        onClick={() => setExpanded(!expanded)}
      >
        {/* Phase indicator */}
        <span className={`w-2.5 h-2.5 rounded-full ${phaseColor} ${
          snapshot.phase === 'coding' || snapshot.phase === 'thinking' ? 'animate-pulse' : ''
        }`} />

        {/* Name + phase */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium truncate">{snapshot.role_name}</span>
            <span className="text-xs text-gray-400">{phaseLabel}</span>
          </div>
          {snapshot.current_task && (
            <p className="text-xs text-gray-500 truncate">{snapshot.current_task}</p>
          )}
        </div>

        {/* Progress */}
        <div className="flex items-center gap-2 shrink-0">
          <div className="w-16 bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
            <div
              className={`h-1.5 rounded-full ${phaseColor} transition-all duration-300`}
              style={{ width: `${snapshot.progress_pct}%` }}
            />
          </div>
          <span className="text-xs text-gray-500 w-8 text-right">{snapshot.progress_pct}%</span>
        </div>

        {/* Expand arrow */}
        <span className="text-gray-400 text-xs">{expanded ? '▼' : '▶'}</span>
      </button>

      {/* Expanded details */}
      {expanded && (
        <div className="px-3 pb-3 space-y-2 border-t border-gray-100 dark:border-gray-700 pt-2">
          {snapshot.summary && (
            <div>
              <span className="text-xs font-medium text-gray-500">摘要</span>
              <p className="text-xs text-gray-600 dark:text-gray-400">{snapshot.summary}</p>
            </div>
          )}
          {snapshot.files_touched.length > 0 && (
            <div>
              <span className="text-xs font-medium text-gray-500">修改文件 ({snapshot.files_touched.length})</span>
              <div className="flex flex-wrap gap-1 mt-1">
                {snapshot.files_touched.slice(0, 8).map(f => (
                  <span key={f} className="text-xs bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded truncate max-w-[200px]">
                    {f}
                  </span>
                ))}
                {snapshot.files_touched.length > 8 && (
                  <span className="text-xs text-gray-400">+{snapshot.files_touched.length - 8}</span>
                )}
              </div>
            </div>
          )}
          <div className="flex gap-4 text-xs text-gray-400">
            <span>执行次数: {snapshot.execution_count}</span>
            <span>更新: {new Date(snapshot.updated_at).toLocaleTimeString()}</span>
          </div>
        </div>
      )}
    </div>
  );
}
