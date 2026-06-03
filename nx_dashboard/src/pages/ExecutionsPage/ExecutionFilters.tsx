import { useMemo, useState } from 'react';
import type { Execution } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useWorkflowStore } from '@/stores/workflowStore';

export interface ExecutionFilterState {
  teamId: string;
  projectId: string;
  workflowId: string;
  status: string;
  dateFrom: string;
  dateTo: string;
}

const DEFAULT: ExecutionFilterState = {
  teamId: '',
  projectId: '',
  workflowId: '',
  status: '',
  dateFrom: '',
  dateTo: '',
};

export function useExecutionFilters(executions: Execution[]) {
  const [filters, setFilters] = useState<ExecutionFilterState>(DEFAULT);
  const teams = useTeamStore((s) => s.teams);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const workflows = useWorkflowStore((s) => s.workflows);

  const filtered = useMemo(() => {
    return executions.filter((e) => {
      if (filters.teamId && e.team_id !== filters.teamId) return false;
      if (filters.projectId && e.project_id !== filters.projectId) return false;
      if (filters.workflowId && e.workflow_id !== filters.workflowId) return false;
      if (filters.status && e.status !== filters.status) return false;
      if (filters.dateFrom) {
        const from = new Date(filters.dateFrom).getTime();
        const started = e.started_at ? new Date(e.started_at).getTime() : 0;
        if (started < from) return false;
      }
      if (filters.dateTo) {
        const to = new Date(filters.dateTo).getTime() + 86400000;
        const started = e.started_at ? new Date(e.started_at).getTime() : 0;
        if (started > to) return false;
      }
      return true;
    });
  }, [executions, filters]);

  return { filters, setFilters, filtered, teams, workspaces, workflows, reset: () => setFilters(DEFAULT) };
}

interface ExecutionFiltersProps {
  filters: ExecutionFilterState;
  onChange: (next: ExecutionFilterState) => void;
  onReset: () => void;
  teams: { id: string; name: string }[];
  workspaces: { id: string; name: string }[];
  workflows: { id: string; name: string }[];
}

export function ExecutionFiltersBar({
  filters,
  onChange,
  onReset,
  teams,
  workspaces,
  workflows,
}: ExecutionFiltersProps) {
  const set = (patch: Partial<ExecutionFilterState>) => onChange({ ...filters, ...patch });

  return (
    <div className="flex flex-wrap items-end gap-2 p-3 rounded-xl border border-border/50 bg-muted/20 text-xs">
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">团队</span>
        <select
          className="rounded-md border border-border/50 bg-background px-2 py-1.5 min-w-[120px]"
          value={filters.teamId}
          onChange={(e) => set({ teamId: e.target.value })}
        >
          <option value="">全部</option>
          {teams.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">工作区</span>
        <select
          className="rounded-md border border-border/50 bg-background px-2 py-1.5 min-w-[120px]"
          value={filters.projectId}
          onChange={(e) => set({ projectId: e.target.value })}
        >
          <option value="">全部</option>
          {workspaces.map((w) => (
            <option key={w.id} value={w.id}>
              {w.name}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">产线</span>
        <select
          className="rounded-md border border-border/50 bg-background px-2 py-1.5 min-w-[120px]"
          value={filters.workflowId}
          onChange={(e) => set({ workflowId: e.target.value })}
        >
          <option value="">全部</option>
          {workflows.map((w) => (
            <option key={w.id} value={w.id}>
              {w.name}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">状态</span>
        <select
          className="rounded-md border border-border/50 bg-background px-2 py-1.5"
          value={filters.status}
          onChange={(e) => set({ status: e.target.value })}
        >
          <option value="">全部</option>
          {['running', 'paused', 'completed', 'failed', 'cancelled', 'pending'].map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">起</span>
        <input
          type="date"
          className="rounded-md border border-border/50 bg-background px-2 py-1.5"
          value={filters.dateFrom}
          onChange={(e) => set({ dateFrom: e.target.value })}
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-muted-foreground">止</span>
        <input
          type="date"
          className="rounded-md border border-border/50 bg-background px-2 py-1.5"
          value={filters.dateTo}
          onChange={(e) => set({ dateTo: e.target.value })}
        />
      </label>
      <button type="button" onClick={onReset} className="px-2 py-1.5 rounded-md hover:bg-accent">
        重置
      </button>
    </div>
  );
}
