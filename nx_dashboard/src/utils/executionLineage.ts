import { api, type ArtifactRecord, type ArtifactSummary } from '@/api/client';
import type { Execution } from '@/stores/executionStore';

/** Run + 父 Run（重试 lineage）的 execution id，按时间从旧到新 */
export function lineageExecutionIds(execution: Execution): string[] {
  const ids: string[] = [];
  if (execution.resumed_from) ids.push(execution.resumed_from);
  ids.push(execution.id);
  return ids;
}

export interface MergedArtifacts {
  summary: ArtifactSummary[];
  files: ArtifactRecord[];
}

/** 合并 lineage 内各 Run 的产物（同路径保留较新 Run 的记录） */
export async function loadMergedArtifacts(execution: Execution): Promise<MergedArtifacts> {
  const ids = lineageExecutionIds(execution);
  const summaries = await Promise.all(ids.map((id) => api.getArtifactsSummary(id)));
  const lists = await Promise.all(ids.map((id) => api.listArtifacts(id)));

  const mergedSummary = summaries.flatMap((s) => (Array.isArray(s) ? s : []));
  const fileMap = new Map<string, ArtifactRecord>();
  for (const list of lists) {
    if (!Array.isArray(list)) continue;
    for (const f of list) {
      fileMap.set(f.relative_path, f);
    }
  }

  return {
    summary: mergedSummary,
    files: [...fileMap.values()],
  };
}

/** 计进度时忽略 agent:* 伪阶段名 */
export function workflowStageNamesFromResults(execution: Execution): Set<string> {
  return new Set(
    (execution.stage_results ?? [])
      .map((s) => s.stage_name)
      .filter((name) => !name.startsWith('agent:')),
  );
}
