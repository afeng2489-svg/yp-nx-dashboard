import { API_BASE_URL } from '@/api/constants';
import { suggestWorkflowName } from '@/data/factoryQuickStart';
import { useSettingsStore } from '@/stores/settingsStore';

export interface QuickRunOptions {
  prompt: string;
  teamId?: string;
  projectId?: string;
  workflowName?: string;
  retryFromStage?: string;
  skipQualityGateForStage?: string;
  retryExecutionId?: string;
}

export interface QuickRunResult {
  ok: boolean;
  executionId?: string;
  error?: string;
  workflowName?: string;
}

function factoryPrefs() {
  const f = useSettingsStore.getState().factory;
  return {
    approvalPolicy: f.approvalPolicy,
    textLaneCostMode: f.textLaneCostMode,
  };
}

export async function retryFactoryStage(opts: {
  executionId: string;
  fromStage?: string;
  skipQualityGateForStage?: string;
}): Promise<QuickRunResult> {
  const prefs = factoryPrefs();
  const res = await fetch(`${API_BASE_URL}/api/v1/executions/${opts.executionId}/retry-stage`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      from_stage: opts.fromStage,
      approval_policy: prefs.approvalPolicy,
      text_lane_cost_mode: prefs.textLaneCostMode,
      skip_quality_gate_for_stage: opts.skipQualityGateForStage,
    }),
  });
  const data = await res.json();
  if (!data.ok) {
    return { ok: false, error: data.error ?? '重试失败' };
  }
  return {
    ok: true,
    executionId: data.data?.execution_id as string | undefined,
  };
}

export async function runFactoryQuickPrompt(opts: QuickRunOptions): Promise<QuickRunResult> {
  const prompt = opts.prompt.trim();
  if (opts.retryExecutionId) {
    return retryFactoryStage({
      executionId: opts.retryExecutionId,
      fromStage: opts.retryFromStage,
      skipQualityGateForStage: opts.skipQualityGateForStage,
    });
  }
  if (!prompt) return { ok: false, error: 'prompt 不能为空' };

  const workflowName = opts.workflowName ?? suggestWorkflowName(prompt);
  const prefs = factoryPrefs();

  const res = await fetch(`${API_BASE_URL}/api/v1/quick-run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      prompt,
      team_id: opts.teamId,
      project_id: opts.projectId,
      workflow_name: workflowName,
      approval_policy: prefs.approvalPolicy,
      text_lane_cost_mode: prefs.textLaneCostMode,
      retry_from_stage: opts.retryFromStage,
    }),
  });
  const data = await res.json();
  if (!data.ok) {
    return { ok: false, error: data.error ?? '启动失败' };
  }
  return {
    ok: true,
    executionId: data.data?.execution_id as string | undefined,
    workflowName: data.data?.workflow_name ?? workflowName,
  };
}
