import { useWorkflowStore } from '@/stores/workflowStore';

// 计算执行持续时间
export function formatDuration(startedAt?: string, finishedAt?: string): string {
  if (!startedAt) return '-';
  const start = new Date(startedAt).getTime();
  const end = finishedAt ? new Date(finishedAt).getTime() : Date.now();
  const duration = Math.floor((end - start) / 1000);

  if (duration < 60) return `${duration}秒`;
  if (duration < 3600) return `${Math.floor(duration / 60)}分${duration % 60}秒`;
  return `${Math.floor(duration / 3600)}小时${Math.floor((duration % 3600) / 60)}分`;
}

// 格式化时间
export function formatTime(dateStr?: string): string {
  if (!dateStr) return '-';
  return new Date(dateStr).toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/**
 * 把 workflow_id 渲染成"工作流名字"。找不到时回退显示截短 UUID。
 * 在组件渲染期间调用 useWorkflowStore，自动响应 workflows 变化。
 */
export function useWorkflowName() {
  const workflows = useWorkflowStore((s) => s.workflows);
  return (workflowId: string): string => {
    const found = workflows.find((w) => w.id === workflowId);
    if (found) return found.name;
    // 兜底：找不到（工作流被删 或 还没加载完）显示前 8 位 UUID
    return workflowId.length > 12 ? `${workflowId.slice(0, 8)}...` : workflowId;
  };
}
