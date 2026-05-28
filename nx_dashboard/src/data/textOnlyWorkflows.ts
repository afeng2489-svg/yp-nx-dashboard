/** AF-MM-03：无 Claude CLI 时可跑的文本类产线 */

export const TEXT_ONLY_WORKFLOW_NAMES = [
  'writing-plans',
  'review-cycle',
] as const;

export type TextOnlyWorkflowName = (typeof TEXT_ONLY_WORKFLOW_NAMES)[number];

export function isTextOnlyWorkflow(workflowName?: string): boolean {
  if (!workflowName) return false;
  return (TEXT_ONLY_WORKFLOW_NAMES as readonly string[]).includes(workflowName);
}

export function requiresClaudeCliForWorkflow(workflowName?: string): boolean {
  return !isTextOnlyWorkflow(workflowName);
}
