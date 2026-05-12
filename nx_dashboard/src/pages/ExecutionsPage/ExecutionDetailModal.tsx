import { useState } from 'react';
import { PauseCircle, X, Activity, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Execution } from '@/stores/executionStore';
import { ArtifactsPanel } from '@/components/execution/ArtifactsPanel';
import { STATUS_CONFIG } from './constants';
import { formatTime, formatDuration, useWorkflowName } from './utils';
import { StageResultCard } from './StageResultCard';
import { ExecutionLogs } from './ExecutionLogs';
import { GitTab } from './GitTab';

export interface ExecutionDetailModalProps {
  execution: Execution;
  onClose: () => void;
  onCancel: (id: string) => void;
}

export function ExecutionDetailModal({ execution, onClose, onCancel }: ExecutionDetailModalProps) {
  const [activeTab, setActiveTab] = useState<'stages' | 'logs' | 'artifacts' | 'git'>('stages');
  const [expandedStages, setExpandedStages] = useState<Set<string>>(new Set());
  const workflowName = useWorkflowName();

  const config =
    STATUS_CONFIG[execution.status as keyof typeof STATUS_CONFIG] ?? STATUS_CONFIG.pending;
  const Icon = config.icon;

  const toggleStage = (stageName: string) => {
    setExpandedStages((prev) => {
      const next = new Set(prev);
      if (next.has(stageName)) {
        next.delete(stageName);
      } else {
        next.add(stageName);
      }
      return next;
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-gradient-to-b from-black/50 to-black/70 backdrop-blur-sm">
      <div className="bg-card rounded-2xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col animate-scale-in border border-border/50 overflow-hidden">
        {/* 弹窗头部 */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-indigo-500/5 to-purple-500/5">
          <div className="flex items-center gap-4">
            <div
              className={cn('p-2.5 rounded-xl bg-gradient-to-br ', config.gradient, 'shadow-lg')}
            >
              <Icon className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">执行详情</h2>
              <p className="text-sm text-muted-foreground font-mono">ID: {execution.id}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {execution.status === 'running' && (
              <button
                onClick={() => onCancel(execution.id)}
                className="px-4 py-2 text-sm rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white shadow-lg shadow-red-500/25 hover:shadow-red-500/40 transition-all"
              >
                取消执行
              </button>
            )}
            {execution.status === 'paused' && (
              <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30">
                <PauseCircle className="w-4 h-4 text-amber-500 animate-pulse" />
                <span className="text-sm text-amber-600 font-medium">等待用户输入</span>
              </div>
            )}
            <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* 执行信息 */}
        <div className="px-6 py-4 bg-gradient-to-r from-indigo-500/5 to-purple-500/5 border-b border-border/50">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">工作流</p>
              <p className="font-semibold truncate" title={execution.workflow_id}>
                {workflowName(execution.workflow_id)}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">状态</p>
              <span
                className={cn(
                  'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium',
                  'bg-gradient-to-r ' + config.gradient,
                  'text-white shadow-md',
                )}
              >
                {config.label}
                {execution.resumed_from && (
                  <span className="ml-1.5 px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-600 font-medium">
                    已恢复
                  </span>
                )}
              </span>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">开始时间</p>
              <p className="font-medium text-sm">{formatTime(execution.started_at)}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">持续时间</p>
              <p className="font-medium text-sm">
                {formatDuration(execution.started_at, execution.finished_at)}
              </p>
            </div>
          </div>
        </div>

        {/* Tab 切换 */}
        <div className="flex border-b border-border/50">
          {(['stages', 'logs', 'artifacts', 'git'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={cn(
                'flex-1 px-4 py-3 text-sm font-medium transition-all relative',
                activeTab === tab
                  ? 'text-indigo-600'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {tab === 'stages'
                ? '阶段结果'
                : tab === 'logs'
                  ? '执行日志'
                  : tab === 'artifacts'
                    ? '产物变更'
                    : 'Git'}
              {activeTab === tab && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-indigo-500 to-purple-500" />
              )}
            </button>
          ))}
        </div>

        {/* 内容区域 */}
        <div className="flex-1 overflow-auto p-6">
          {activeTab === 'stages' ? (
            <div className="space-y-3">
              {!execution.stage_results || execution.stage_results.length === 0 ? (
                <div className="text-center py-12">
                  <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
                    <Activity className="w-8 h-8 text-indigo-500" />
                  </div>
                  <p className="text-muted-foreground">暂无阶段数据</p>
                </div>
              ) : (
                execution.stage_results?.map((result, idx) => (
                  <StageResultCard
                    key={idx}
                    result={result}
                    isExpanded={expandedStages.has(result.stage_name)}
                    onToggle={() => toggleStage(result.stage_name)}
                  />
                ))
              )}
            </div>
          ) : activeTab === 'logs' ? (
            <ExecutionLogs executionId={execution.id} />
          ) : activeTab === 'artifacts' ? (
            <ArtifactsPanel executionId={execution.id} />
          ) : (
            <GitTab executionId={execution.id} executionStatus={execution.status} />
          )}
        </div>

        {/* 错误信息 */}
        {execution.error && (
          <div className="px-6 py-4 border-t border-border/50 bg-gradient-to-r from-red-500/5 to-rose-500/5">
            <div className="flex items-start gap-3 p-3 rounded-xl bg-red-500/10 border border-red-500/20">
              <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
              <div>
                <p className="font-medium text-red-600">执行错误</p>
                <p className="text-sm mt-1 text-red-600/80">{execution.error}</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
