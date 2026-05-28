import { Clock, CheckCircle, Loader2, PauseCircle, XCircle, AlertCircle } from 'lucide-react';

// 状态配置
export const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    gradient: 'from-slate-400 to-gray-500',
    label: '等待中',
  },
  running: {
    icon: Loader2,
    gradient: 'from-blue-500 to-indigo-500',
    label: '运行中',
  },
  paused: {
    icon: PauseCircle,
    gradient: 'from-amber-400 to-orange-500',
    label: '等待输入',
  },
  completed: {
    icon: CheckCircle,
    gradient: 'from-emerald-500 to-green-500',
    label: '已完成',
  },
  failed: {
    icon: XCircle,
    gradient: 'from-red-500 to-rose-500',
    label: '失败',
  },
  cancelled: {
    icon: XCircle,
    gradient: 'from-slate-400 to-gray-500',
    label: '已取消',
  },
  interrupted: {
    icon: AlertCircle,
    gradient: 'from-orange-500 to-amber-500',
    label: '已中断',
  },
} as const;

export const STATUS_ACCENT_BORDER = {
  pending: 'border-l-slate-400',
  running: 'border-l-blue-500',
  paused: 'border-l-amber-500',
  completed: 'border-l-emerald-500',
  failed: 'border-l-red-500',
  cancelled: 'border-l-slate-500',
  interrupted: 'border-l-orange-500',
} as const;

export const STATUS_TEXT = {
  pending: 'text-slate-400',
  running: 'text-blue-400',
  paused: 'text-amber-400',
  completed: 'text-emerald-400',
  failed: 'text-red-400',
  cancelled: 'text-slate-400',
  interrupted: 'text-orange-400',
} as const;
export const WORKFLOW_OPERATIONS = [
  { key: '1', action: '创建', desc: '点击"新建工作流"进入编辑器' },
  { key: '2', action: '编辑', desc: '从列表点击编辑图标，或在画布上拖拽节点' },
  { key: '3', action: '保存', desc: '点击"保存"按钮保存到后端' },
  { key: '4', action: '执行', desc: '点击播放图标执行工作流' },
  { key: '5', action: '导入/导出', desc: '使用 Export 按钮导出 JSON，可用于备份或分享' },
] as const;
