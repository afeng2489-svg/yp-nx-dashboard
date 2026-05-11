import { useState, useEffect } from 'react';
import { GitBranch } from 'lucide-react';
import { useSnapshotStore } from '@/stores/snapshotStore';
import ProjectProgressDashboard from '@/components/team/ProjectProgressDashboard';
import CrashRecoveryDialog from '@/components/team/CrashRecoveryDialog';
import ProcessResourceBar from '@/components/team/ProcessResourceBar';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

export interface TeamEvolutionSectionProps {
  projectId: string;
}

/** Collapsible Team Evolution section: progress + resources + crash recovery */
export function TeamEvolutionSection({ projectId }: TeamEvolutionSectionProps) {
  const [expanded, setExpanded] = useState(false);
  const { progress, fetchProgress } = useSnapshotStore();
  const [approving, setApproving] = useState(false);

  const pipelineId = progress?.pipeline_id;
  const needsApproval = progress?.overall_phase === 'waiting_for_approval';

  // 轮询 pipeline 状态（等待审批时加快频率）
  useEffect(() => {
    if (!projectId) return;
    const interval = setInterval(() => fetchProgress(projectId), needsApproval ? 5000 : 15000);
    return () => clearInterval(interval);
  }, [projectId, needsApproval, fetchProgress]);

  const handleApprove = async () => {
    if (!pipelineId) return;
    setApproving(true);
    try {
      await fetch(`${API_BASE}/api/v1/pipelines/${pipelineId}/approve`, { method: 'POST' });
      await fetchProgress(projectId);
    } finally {
      setApproving(false);
    }
  };

  const handleReject = async () => {
    if (!pipelineId) return;
    const reason = window.prompt('拒绝原因（可选）');
    if (reason === null) return;
    setApproving(true);
    try {
      await fetch(`${API_BASE}/api/v1/pipelines/${pipelineId}/reject`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ reason }),
      });
      await fetchProgress(projectId);
    } finally {
      setApproving(false);
    }
  };

  return (
    <div className="bg-card rounded-lg border overflow-hidden">
      <button
        className="w-full flex items-center gap-3 px-4 py-2.5 hover:bg-accent/50 transition-colors text-left"
        onClick={() => setExpanded(!expanded)}
      >
        <GitBranch className="w-4 h-4 text-indigo-500" />
        <span className="text-sm font-medium flex-1">团队进化面板</span>
        {needsApproval && (
          <span className="text-xs bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded-full font-medium">
            等待审批
          </span>
        )}
        <ProcessResourceBar />
        <span className="text-xs text-muted-foreground">{expanded ? '收起 ▲' : '展开 ▼'}</span>
      </button>

      {needsApproval && pipelineId && (
        <div className="border-t px-4 py-3 bg-yellow-50 dark:bg-yellow-900/20 flex items-center gap-3">
          <span className="text-sm flex-1 text-yellow-800 dark:text-yellow-200">
            架构设计已完成，请审批后继续执行
          </span>
          <button
            onClick={handleApprove}
            disabled={approving}
            className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
          >
            通过
          </button>
          <button
            onClick={handleReject}
            disabled={approving}
            className="px-3 py-1 text-sm bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
          >
            拒绝
          </button>
        </div>
      )}

      {expanded && (
        <div className="border-t p-4 space-y-4 max-h-[400px] overflow-y-auto">
          <ProjectProgressDashboard projectId={projectId} />
          <CrashRecoveryDialog projectId={projectId} />
        </div>
      )}
    </div>
  );
}
