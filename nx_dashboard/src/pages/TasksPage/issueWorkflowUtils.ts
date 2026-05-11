import type { Issue } from '@/stores/issueStore';
import { API_BASE_URL } from '@/api/constants';
import { showError, showSuccess } from '@/lib/toast';

interface Workflow {
  id: string;
  name: string;
  definition?: unknown;
}

export async function triggerIssueWorkflow(
  issue: Issue,
  action: 'plan' | 'queue' | 'execute',
  workflows: Workflow[],
  onComplete?: () => void,
): Promise<void> {
  const workflowNames: Record<string, string> = {
    plan: 'issue-plan',
    queue: 'issue-queue',
    execute: 'issue-execute',
  };
  const wfName = workflowNames[action];
  const wf = workflows.find((w) => w.name === wfName);
  if (!wf) {
    showError(`工作流 ${wfName} 未找到，请确认已导入`);
    return;
  }

  try {
    const detailRes = await fetch(`${API_BASE_URL}/api/v1/workflows/${wf.id}`);
    const wfFull = await detailRes.json();
    const variables = action === 'queue' ? { issue_ids: issue.id } : { issue_id: issue.id };

    const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${wf.id}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_yaml: JSON.stringify(wfFull.definition ?? {}), variables }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    showSuccess(`${wfName} 工作流已触发`);
    onComplete?.();
  } catch (e) {
    showError(`触发失败：${e}`);
  }
}

export async function triggerDiscoverWorkflow(workflows: Workflow[]): Promise<void> {
  const wf = workflows.find((w) => w.name === 'issue-discover');
  if (!wf) {
    showError('工作流 issue-discover 未找到');
    return;
  }
  try {
    const detailRes = await fetch(`${API_BASE_URL}/api/v1/workflows/${wf.id}`);
    const wfFull = await detailRes.json();
    const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${wf.id}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        workflow_yaml: JSON.stringify(wfFull.definition ?? {}),
        variables: { target: '.' },
      }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    showSuccess('issue-discover 工作流已触发');
  } catch (e) {
    showError(`触发失败：${e}`);
  }
}
