import type { Execution } from '@/stores/executionStore';

export interface QualityGateFailureInfo {
  stageName: string;
  failedChecks: string[];
  retryCount: number;
}

/** 检测 Run 是否因质量门失败 */
export function detectQualityGateFailure(execution: Execution): QualityGateFailureInfo | null {
  if (execution.status !== 'failed') return null;

  for (let i = (execution.stage_results?.length ?? 0) - 1; i >= 0; i -= 1) {
    const sr = execution.stage_results![i];
    const qg = sr.quality_gate_result;
    if (qg && qg.passed === false) {
      return {
        stageName: sr.stage_name,
        failedChecks: qg.checks?.filter((c) => !c.passed).map((c) => c.cmd) ?? [],
        retryCount: qg.retry_count ?? 0,
      };
    }
  }

  if (execution.error?.includes('质量门')) {
    return {
      stageName: execution.current_stage ?? execution.stage_results?.at(-1)?.stage_name ?? '未知阶段',
      failedChecks: [],
      retryCount: 0,
    };
  }

  return null;
}
