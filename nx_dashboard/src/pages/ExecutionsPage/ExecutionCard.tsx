import { CheckCircle, XCircle, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Execution } from '@/stores/executionStore';
import { ExecutionTokenBadge } from '@/components/dashboard';
import { STATUS_CONFIG } from './constants';
import { formatTime, formatDuration, useWorkflowName } from './utils';

export interface ExecutionCardProps {
  execution: Execution;
  onClick: () => void;
  onCancel: (id: string) => void;
  selected: boolean;
  onSelect: (id: string) => void;
}

export function ExecutionCard({
  execution,
  onClick,
  onCancel,
  selected,
  onSelect,
}: ExecutionCardProps) {
  const config = STATUS_CONFIG[execution.status];
  const Icon = config.icon;
  const workflowName = useWorkflowName();

  return (
    <div
      onClick={onClick}
      className={cn(
        'bg-card rounded-2xl border p-5 cursor-pointer relative',
        'hover:shadow-lg hover:shadow-primary/5 transition-all duration-200 hover:-translate-y-0.5 group',
        selected
          ? 'border-indigo-500/60 bg-indigo-500/5'
          : 'border-border/50 hover:border-primary/20',
      )}
    >
      {/* 多选 checkbox */}
      <div
        className="absolute top-3 left-3 z-10"
        onClick={(e) => {
          e.stopPropagation();
          onSelect(execution.id);
        }}
      >
        <div
          className={cn(
            'w-5 h-5 rounded border-2 flex items-center justify-center transition-colors',
            selected
              ? 'bg-indigo-500 border-indigo-500'
              : 'border-border/60 hover:border-indigo-400',
          )}
        >
          {selected && <CheckCircle className="w-3 h-3 text-white" />}
        </div>
      </div>

      <div className="flex items-start justify-between mb-4 pl-7">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'p-2.5 rounded-xl bg-gradient-to-br',
              config.gradient,
              'shadow-lg group-hover:scale-110 transition-transform duration-200',
            )}
          >
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <h3
              className="font-semibold group-hover:text-indigo-600 transition-colors"
              title={execution.workflow_id}
            >
              {workflowName(execution.workflow_id)}
            </h3>
            <p className="text-xs text-muted-foreground font-mono">
              ID: {execution.id.slice(0, 8)}...
            </p>
          </div>
        </div>
        <span
          className={cn(
            'px-3 py-1.5 rounded-full text-xs font-medium shadow-md',
            'bg-gradient-to-r ' + config.gradient,
            'text-white',
          )}
        >
          {config.label}
          {execution.resumed_from && (
            <span className="ml-1.5 px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-600 font-medium">
              已恢复
            </span>
          )}
        </span>
        {execution.stage_results && execution.stage_results.length > 0 && (
          <span className="px-2 py-1 rounded-full text-xs font-medium bg-indigo-500/10 text-indigo-600 border border-indigo-500/20">
            {execution.stage_results.length} 阶段
          </span>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm pl-7">
        <div className="space-y-1">
          <p className="text-muted-foreground text-xs">开始时间</p>
          <p className="font-medium text-sm">{formatTime(execution.started_at)}</p>
        </div>
        <div className="space-y-1">
          <p className="text-muted-foreground text-xs">持续时间</p>
          <p className="font-medium text-sm">
            {formatDuration(execution.started_at, execution.finished_at)}
          </p>
        </div>
      </div>

      {execution.stage_results && execution.stage_results.length > 0 && (
        <div className="mt-4 pt-4 border-t border-border/50 pl-7">
          <p className="text-xs text-muted-foreground mb-2">
            阶段进度 ({execution.stage_results.length})
          </p>
          <div className="flex items-center gap-2">
            {execution.stage_results.slice(0, 3).map((result, idx) => (
              <span
                key={idx}
                className="px-2 py-1 text-xs bg-gradient-to-r from-emerald-500/10 to-green-500/10 text-emerald-600 rounded-lg border border-emerald-500/20"
              >
                {result.stage_name}
              </span>
            ))}
            {execution.stage_results.length > 3 && (
              <span className="text-xs text-muted-foreground">
                +{execution.stage_results.length - 3} 更多
              </span>
            )}
            <ExecutionTokenBadge executionId={execution.id} />
          </div>
        </div>
      )}

      {execution.error && (
        <div className="mt-4 p-3 rounded-xl bg-gradient-to-r from-red-500/10 to-rose-500/10 border border-red-500/20 flex items-start gap-2 pl-7">
          <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5" />
          <span className="text-sm text-red-600 line-clamp-2">{execution.error}</span>
        </div>
      )}

      {execution.status === 'running' && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onCancel(execution.id);
          }}
          className="absolute top-3 right-3 p-1.5 rounded-lg bg-red-500/10 text-red-500 hover:bg-red-500/20 transition-colors"
        >
          <XCircle className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}
