import { Terminal, Wifi } from 'lucide-react';
import { pipelineLabelForWorkflow } from '@/data/workflowPipelines';
import { isTextOnlyWorkflow } from '@/data/textOnlyWorkflows';

export interface ExecutionLaneBadgeProps {
  workflowName?: string;
  stageName?: string;
}

/** AF-MM-02：执行车道展示（代码 CLI vs API 文本） */
export function ExecutionLaneBadge({ workflowName, stageName }: ExecutionLaneBadgeProps) {
  const isSummary =
    stageName?.includes('摘要') ||
    stageName?.includes('计划') ||
    stageName === '编写实现计划';
  const textOnly = isTextOnlyWorkflow(workflowName);
  const apiLane = isSummary || textOnly;

  return (
    <div
      className="inline-flex items-center gap-2 text-[10px] text-muted-foreground"
      data-testid="execution-lane-badge"
    >
      {apiLane ? (
        <span className="inline-flex items-center gap-1 text-sky-700 dark:text-sky-300">
          <Wifi className="h-3 w-3" />
          文本车道 · API
        </span>
      ) : (
        <span className="inline-flex items-center gap-1 text-emerald-700 dark:text-emerald-300">
          <Terminal className="h-3 w-3" />
          代码车道 · Claude CLI
        </span>
      )}
      {workflowName && (
        <span>· {pipelineLabelForWorkflow(workflowName)}</span>
      )}
    </div>
  );
}
