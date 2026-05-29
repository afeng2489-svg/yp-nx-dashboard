import { detectQualityGateFailure } from '@/data/qualityGateRecovery';
import type { Execution } from '@/stores/executionStore';

export interface RunNextStepAction {
  label: string;
  kind: 'prefill' | 'navigate' | 'retry' | 'run';
  /** prefill / run / retry 用的 prompt */
  prompt?: string;
  /** run / retry 时覆盖 workflow */
  workflowName?: string;
  /** retry 时从失败 execution 重试 */
  retryExecutionId?: string;
  retryFromStage?: string;
  /** AF-UX-09：跳过指定 stage 的质量门 */
  skipQualityGateForStage?: string;
  /** navigate 目标 */
  href?: string;
}

export interface RunNextSteps {
  title: string;
  summary?: string;
  filesChanged?: number;
  primary: RunNextStepAction;
  secondary?: RunNextStepAction;
  tertiary?: RunNextStepAction;
}

const DISMISS_PREFIX = 'factory-dismissed-run:';

let dismissEpoch = 0;
const dismissSubscribers = new Set<() => void>();

function notifyDismissChange() {
  dismissEpoch += 1;
  dismissSubscribers.forEach((fn) => fn());
}

/** 供 useSyncExternalStore：关闭 Banner 后立即刷新 UI */
export function subscribeRunOutcomeDismiss(onStoreChange: () => void): () => void {
  dismissSubscribers.add(onStoreChange);
  return () => dismissSubscribers.delete(onStoreChange);
}

export function getRunOutcomeDismissEpoch(): number {
  return dismissEpoch;
}

export function dismissRunOutcome(executionId: string): void {
  try {
    localStorage.setItem(`${DISMISS_PREFIX}${executionId}`, '1');
    notifyDismissChange();
  } catch {
    /* ignore */
  }
}

export function isRunOutcomeDismissed(executionId: string): boolean {
  try {
    return localStorage.getItem(`${DISMISS_PREFIX}${executionId}`) === '1';
  } catch {
    return false;
  }
}

/** 从 execution variables 提取可重跑 prompt */
export function extractRunPrompt(execution: Execution): string {
  const v = execution.variables ?? {};
  for (const key of ['task', 'prompt', 'goal', 'feature_name', 'description']) {
    const val = v[key];
    if (typeof val === 'string' && val.trim()) return val.trim();
  }
  if (typeof v.target === 'string' && v.target.trim()) return v.target.trim();
  return '';
}

function parseSummaryFromStages(execution: Execution): { summary?: string; filesChanged?: number } {
  const stages = execution.stage_results ?? [];
  for (let i = stages.length - 1; i >= 0; i -= 1) {
    for (const out of stages[i].outputs ?? []) {
      const content = out.content ?? out.summary ?? '';
      const jsonMatch = content.match(/```json\s*([\s\S]*?)```/);
      if (jsonMatch) {
        try {
          const parsed = JSON.parse(jsonMatch[1]) as { summary?: string; files_changed?: string[] };
          return {
            summary: parsed.summary,
            filesChanged: parsed.files_changed?.length,
          };
        } catch {
          /* fall through */
        }
      }
      if (content.length > 20 && content.length < 600) {
        return { summary: content.slice(0, 280) };
      }
    }
  }
  return {};
}

export function nextStepsForRun(
  execution: Execution,
  workflowName: string,
): RunNextSteps | null {
  const prompt = extractRunPrompt(execution);
  const { summary, filesChanged } = parseSummaryFromStages(execution);

  if (execution.status === 'failed') {
    const qgFail = detectQualityGateFailure(execution);
    let stage =
      qgFail?.stageName ??
      execution.current_stage ??
      execution.stage_results?.at(-1)?.stage_name;

    if (
      execution.error &&
      (/summary|摘要/i.test(execution.error) ||
        /Agent\s+\S+\s+failed/i.test(execution.error) ||
        execution.error.includes('智能体 summary'))
    ) {
      stage = '交付摘要';
    }

    if (stage?.startsWith('agent:')) {
      stage = stage === 'agent:summary' ? '交付摘要' : stage.replace(/^agent:/, '');
    }

    if (qgFail) {
      const checkHint =
        qgFail.failedChecks.length > 0
          ? `未通过：${qgFail.failedChecks.join('、')}`
          : execution.error;
      return {
        title: `质量门未通过：「${qgFail.stageName}」`,
        summary: checkHint ?? summary,
        primary: {
          label: 'AI 再修一轮',
          kind: 'retry',
          prompt,
          workflowName,
          retryExecutionId: execution.id,
          retryFromStage: qgFail.stageName,
        },
        secondary: {
          label: '打开终端',
          kind: 'navigate',
          href: '/factory?tab=console&drawer=terminal',
        },
        tertiary: {
          label: '跳过此门·高级',
          kind: 'retry',
          prompt,
          workflowName,
          retryExecutionId: execution.id,
          retryFromStage: qgFail.stageName,
          skipQualityGateForStage: qgFail.stageName,
        },
      };
    }

    return {
      title: stage ? `Run 在「${stage}」阶段失败` : 'Run 失败',
      summary: execution.error ?? summary,
      primary: {
        label: stage ? `从「${stage}」重试` : '重试此任务',
        kind: 'retry',
        prompt: prompt || '继续完成上次失败的任务',
        workflowName,
        retryExecutionId: execution.id,
        retryFromStage: stage ?? undefined,
      },
      secondary:
        workflowName !== 'quick-fix'
          ? {
              label: '切换快速修复',
              kind: 'run',
              prompt: prompt || '修复上次失败的任务',
              workflowName: 'quick-fix',
            }
          : {
              label: '打开终端',
              kind: 'navigate',
              href: '/factory?tab=console&drawer=terminal',
            },
    };
  }

  if (execution.status !== 'completed') return null;

  const fileHint =
    filesChanged != null && filesChanged > 0 ? `改了 ${filesChanged} 个文件` : undefined;

  switch (workflowName) {
    case 'greenfield-mvp':
      return {
        title: '项目骨架已就绪',
        summary: summary ?? fileHint,
        filesChanged,
        primary: {
          label: '加第一个功能',
          kind: 'prefill',
          prompt: '在现有骨架上加第一个核心功能',
          workflowName: 'solo-dev',
        },
        secondary: {
          label: '查看产物',
          kind: 'navigate',
          href: '/factory?tab=deliverables',
        },
      };

    case 'writing-plans':
      return {
        title: '实施计划已生成',
        summary: summary ?? '可按计划开始编码',
        primary: {
          label: '开始执行计划',
          kind: 'run',
          prompt: prompt ? `按以下计划开始实现：\n${prompt}` : '按刚生成的计划开始实现',
          workflowName: 'solo-dev',
        },
        secondary: {
          label: '查看产物',
          kind: 'navigate',
          href: '/factory?tab=deliverables',
        },
      };

    case 'quick-fix':
      return {
        title: '修复已完成',
        summary: summary ?? fileHint,
        filesChanged,
        primary: {
          label: '加相关测试',
          kind: 'prefill',
          prompt: '为刚修复的问题补充回归测试',
          workflowName: 'solo-dev',
        },
        secondary: {
          label: '查看 diff',
          kind: 'navigate',
          href: '/factory?tab=deliverables',
        },
      };

    default:
      return {
        title: '交付完成',
        summary: summary ?? fileHint,
        filesChanged,
        primary: {
          label: '加第一个功能',
          kind: 'prefill',
          prompt: '在现有改动基础上继续加功能',
          workflowName: 'solo-dev',
        },
        secondary: {
          label: '查看 diff',
          kind: 'navigate',
          href: '/factory?tab=deliverables',
        },
      };
  }
}
