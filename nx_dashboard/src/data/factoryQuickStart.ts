import { Bug, FileText, Palette, Rocket, Search, Sparkles, ShieldCheck } from 'lucide-react';
import { GOLDEN_PATH_TASK } from '@/services/factoryMetrics';

export const FACTORY_QUICK_LINES = [
  {
    id: 'solo-dev',
    workflowName: 'solo-dev',
    title: '一人全栈',
    description: '规划 → 实现 → 自测 → 审查',
    icon: Rocket,
    gradient: 'from-indigo-500 to-violet-600',
  },
  {
    id: 'quick-fix',
    workflowName: 'quick-fix',
    title: '快速修复',
    description: '定位问题并提交修复',
    icon: Bug,
    gradient: 'from-rose-500 to-pink-500',
  },
  {
    id: 'writing-plans',
    workflowName: 'writing-plans',
    title: '实施计划',
    description: '拆任务、写 TDD 步骤',
    icon: FileText,
    gradient: 'from-sky-500 to-blue-500',
  },
  {
    id: 'golden',
    workflowName: 'solo-dev',
    title: '试用演示 ★',
    description: 'Golden Path 一键：README 快速开始',
    icon: Sparkles,
    gradient: 'from-amber-500 to-orange-500',
    presetTask: GOLDEN_PATH_TASK,
  },
  {
    id: 'code-review',
    workflowName: 'review-cycle',
    title: '代码审查',
    description: 'PR 级审查与改进建议',
    icon: ShieldCheck,
    gradient: 'from-violet-500 to-purple-600',
  },
  {
    id: 'ui-design',
    workflowName: 'ui-ux-design',
    title: 'UI 设计',
    description: '页面结构与视觉方案',
    icon: Palette,
    gradient: 'from-fuchsia-500 to-pink-500',
  },
  {
    id: 'research',
    workflowName: 'investigate',
    title: '技术调研',
    description: '问题根因与方案对比',
    icon: Search,
    gradient: 'from-teal-500 to-cyan-500',
  },
] as const;

/** 根据用户输入推荐工作流名（规则，非 LLM） */
export function suggestWorkflowName(prompt: string): string {
  const lower = prompt.toLowerCase();
  if (/bug|fix|修复|报错|崩溃/.test(lower)) return 'quick-fix';
  if (/新建|从零|搭.*项目|做一个.*app|greenfield|mvp/.test(lower)) return 'greenfield-mvp';
  if (/计划|plan|拆解/.test(lower)) return 'writing-plans';
  if (/审查|review|code review/.test(lower)) return 'review-cycle';
  if (/ui|设计|界面|ux/.test(lower)) return 'ui-ux-design';
  if (/调研|investigate|research|根因/.test(lower)) return 'investigate';
  return 'solo-dev';
}
