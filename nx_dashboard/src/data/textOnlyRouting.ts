/** AF-MM-03：无 CLI 时自动路由到文本产线 */

import { suggestWorkflowName } from '@/data/factoryQuickStart';
import { isTextOnlyWorkflow, TEXT_ONLY_WORKFLOW_NAMES } from '@/data/textOnlyWorkflows';

export function pickTextOnlyWorkflowForPrompt(prompt: string): string {
  const lower = prompt.toLowerCase();
  if (/审查|review|code review/i.test(prompt)) return 'review-cycle';
  if (/计划|plan|方案/i.test(prompt)) return 'writing-plans';
  const suggested = suggestWorkflowName(prompt);
  if (isTextOnlyWorkflow(suggested)) return suggested;
  return 'writing-plans';
}

export function autoRouteWorkflowWhenNoCli(
  workflowName: string,
  prompt: string,
  cliReady: boolean | null,
): { workflowName: string; autoRouted: boolean; hint?: string } {
  if (cliReady !== false) return { workflowName, autoRouted: false };
  if (isTextOnlyWorkflow(workflowName)) return { workflowName, autoRouted: false };
  const picked = pickTextOnlyWorkflowForPrompt(prompt);
  return {
    workflowName: picked,
    autoRouted: true,
    hint: `无 Claude CLI，已切换到文本产线「${picked}」（${TEXT_ONLY_WORKFLOW_NAMES.join(' / ')}）`,
  };
}

/** AF-MM-03：文本产线三段展示名 */
export const TEXT_ONLY_PIPELINE_LABEL = '文本产线 · 计划 / 审查 / 摘要';
