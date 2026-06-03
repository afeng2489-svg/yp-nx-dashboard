import { Clock, Coins, FolderOpen, Layers, ShieldCheck } from 'lucide-react';
import { buildLaunchPreview } from '@/data/launchPreview';
import type { StackProfile } from '@/data/stackProfile';

export interface LaunchPreviewBarProps {
  workflowName: string;
  intent?: string;
  stack?: StackProfile;
}

/** AF-UX-07：启动前预览 — 时长 · 阶段 · 路径 · 成本 */
export function LaunchPreviewBar({ workflowName, intent, stack }: LaunchPreviewBarProps) {
  if (!intent?.trim()) return null;

  const preview = buildLaunchPreview(workflowName, stack);

  return (
    <div
      className="rounded-lg border border-border/50 bg-muted/20 px-3 py-2 text-xs text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-1"
      data-testid="launch-preview-bar"
    >
      <span className="inline-flex items-center gap-1">
        <Clock className="h-3.5 w-3.5" />
        {preview.durationHint}
      </span>
      <span className="inline-flex items-center gap-1">
        <Layers className="h-3.5 w-3.5" />
        {preview.stageCount} 阶段 · {preview.workflowLabel}
      </span>
      <span className="inline-flex items-center gap-1">
        <FolderOpen className="h-3.5 w-3.5" />
        可能影响 {preview.affectedPaths}
      </span>
      <span className="inline-flex items-center gap-1">
        <Coins className="h-3.5 w-3.5" />
        预估 CLI ~${preview.estimatedCostUsd.toFixed(2)}
      </span>
      <span className="text-[10px] opacity-80">
        {preview.codeLane}
        {preview.textLane !== '—' ? ` · 文本 ${preview.textLane}` : ''}
      </span>
      {preview.qualityGateHint && (
        <span className="inline-flex items-center gap-1 text-[10px]">
          <ShieldCheck className="h-3.5 w-3.5" />
          门控 {preview.qualityGateHint}
        </span>
      )}
    </div>
  );
}
