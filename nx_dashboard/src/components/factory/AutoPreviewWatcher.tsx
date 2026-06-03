import { useEffect, useRef } from 'react';
import { useExecutionStore } from '@/stores/executionStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { usePreviewLauncher } from '@/lib/usePreviewLauncher';
import { showAction } from '@/lib/toast';

/** 这些工作流会铺出可预览的前端站点（落地页 / 导航站） */
const WEB_GEN_WORKFLOWS = new Set(['landing-page', 'nav-site']);

const TERMINAL = new Set(['completed', 'failed', 'cancelled']);

/**
 * 全局观察者：当落地页 / 导航站类工作流「跑完」时，弹出一个带「▶ 预览效果」
 * 动作的 toast，一键直达预览。只对本会话内「亲眼看到从运行 → 完成」的 Run 触发，
 * 不会对应用启动时加载进来的历史已完成记录误弹。
 */
export function AutoPreviewWatcher() {
  const executions = useExecutionStore((s) => s.executions);
  const workflows = useWorkflowStore((s) => s.workflows);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const { launch } = usePreviewLauncher();

  const prevStatusRef = useRef<Map<string, string>>(new Map());

  useEffect(() => {
    for (const e of executions) {
      const prev = prevStatusRef.current.get(e.id);
      prevStatusRef.current.set(e.id, e.status);

      // 只在「之前见过它处于非终态、这次变成 completed」时触发，
      // 避免对启动时直接加载为 completed 的历史 Run 误弹。
      const justCompleted = e.status === 'completed' && prev !== undefined && !TERMINAL.has(prev);
      if (!justCompleted) continue;

      const wf = workflows.find((w) => w.id === e.workflow_id);
      if (!wf || !WEB_GEN_WORKFLOWS.has(wf.name)) continue;

      const path =
        (typeof e.variables?.project_path === 'string' ? e.variables.project_path : undefined) ??
        currentWorkspace?.root_path ??
        null;

      showAction('站点已生成，可立即预览', {
        label: '▶ 预览效果',
        onClick: () => void launch(path, e.id),
      });
    }
  }, [executions, workflows, currentWorkspace, launch]);

  return null;
}
