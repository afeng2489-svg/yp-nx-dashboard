import { CheckCircle, XCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Execution } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useProjectStore } from '@/stores/projectStore';
import { ExecutionTokenBadge } from '@/components/dashboard';
import { STATUS_CONFIG, STATUS_ACCENT_BORDER, STATUS_TEXT } from './constants';
import { formatTime, formatDuration, useWorkflowName } from './utils';
import { useIsStudioDark } from '@/components/layout/ShellThemeContext';

export interface ExecutionCardProps {
  execution: Execution;
  onClick: () => void;
  onCancel: (id: string) => void;
  isCancelling?: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
}

export function ExecutionCard({
  execution,
  onClick,
  onCancel,
  isCancelling = false,
  selected,
  onSelect,
}: ExecutionCardProps) {
  const isStudio = useIsStudioDark();
  const config = STATUS_CONFIG[execution.status];
  const Icon = config.icon;
  const workflowName = useWorkflowName();
  const teams = useTeamStore((s) => s.teams);
  const projects = useProjectStore((s) => s.projects);
  const teamName = execution.team_id
    ? teams.find((t) => t.id === execution.team_id)?.name
    : undefined;
  const projectName = execution.project_id
    ? projects.find((p) => p.id === execution.project_id)?.name
    : undefined;

  if (isStudio) {
    return (
      <div
        onClick={onClick}
        className={cn(
          'rounded-lg border border-zinc-800 border-l-[3px] p-4 cursor-pointer transition-colors',
          STATUS_ACCENT_BORDER[execution.status],
          selected ? 'bg-zinc-800/80 ring-1 ring-zinc-600' : 'bg-zinc-900/40 hover:bg-zinc-800/50',
        )}
      >
        <div className="flex items-start justify-between gap-3 mb-3">
          <div className="min-w-0 flex-1">
            <h3 className="font-medium text-zinc-100 truncate" title={execution.workflow_id}>
              {workflowName(execution.workflow_id)}
            </h3>
            <p className="text-[11px] text-zinc-500 font-mono mt-0.5">{execution.id.slice(0, 8)}…</p>
          </div>
          <span className={cn('text-xs font-medium shrink-0', STATUS_TEXT[execution.status])}>
            {config.label}
          </span>
        </div>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-zinc-500">
          {teamName && <span>{teamName}</span>}
          {projectName && <span>{projectName}</span>}
          <span>{formatTime(execution.started_at)}</span>
          <span>{formatDuration(execution.started_at, execution.finished_at)}</span>
        </div>
        {(execution.status === 'running' || isCancelling) && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              if (!isCancelling) onCancel(execution.id);
            }}
            disabled={isCancelling}
            className={cn(
              'mt-3 text-xs',
              isCancelling
                ? 'text-zinc-500 cursor-wait'
                : 'text-red-400 hover:text-red-300',
            )}
          >
            {isCancelling ? '取消中…' : '取消运行'}
          </button>
        )}
      </div>
    );
  }

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
        <div className="flex items-center gap-3 min-w-0">
          <div
            className={cn(
              'p-2.5 rounded-xl bg-gradient-to-br shrink-0',
              config.gradient,
              'shadow-lg group-hover:scale-110 transition-transform duration-200',
            )}
          >
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div className="min-w-0">
            <h3
              className="font-semibold group-hover:text-indigo-600 transition-colors truncate"
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
            'px-3 py-1.5 rounded-full text-xs font-medium shadow-md shrink-0',
            'bg-gradient-to-r ' + config.gradient,
            'text-white',
          )}
        >
          {config.label}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm pl-7">
        {(teamName || projectName) && (
          <div className="col-span-2 flex flex-wrap gap-2 text-xs">
            {teamName && (
              <span className="px-2 py-0.5 rounded-md bg-muted text-muted-foreground">团队: {teamName}</span>
            )}
            {projectName && (
              <span className="px-2 py-0.5 rounded-md bg-muted text-muted-foreground">项目: {projectName}</span>
            )}
          </div>
        )}
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
          <div className="flex items-center gap-2 flex-wrap">
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
          <XCircle className="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5" />
          <span className="text-sm text-red-600 line-clamp-2">{execution.error}</span>
        </div>
      )}

      {(execution.status === 'running' || isCancelling) && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            if (!isCancelling) onCancel(execution.id);
          }}
          disabled={isCancelling}
          className={cn(
            'absolute top-3 right-3 p-1.5 rounded-lg transition-colors',
            isCancelling
              ? 'bg-muted text-muted-foreground cursor-wait'
              : 'bg-red-500/10 text-red-500 hover:bg-red-500/20',
          )}
          title={isCancelling ? '取消中…' : '取消运行'}
        >
          <XCircle className={cn('w-4 h-4', isCancelling && 'opacity-50')} />
        </button>
      )}
    </div>
  );
}
