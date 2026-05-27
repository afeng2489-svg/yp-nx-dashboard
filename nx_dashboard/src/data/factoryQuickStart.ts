import { Bug, FileText, Rocket, Sparkles } from 'lucide-react';

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
    title: '试用演示',
    description: 'Golden Path：给 README 加快捷开始',
    icon: Sparkles,
    gradient: 'from-amber-500 to-orange-500',
    presetTask: '给 README.md 增加「快速开始」安装步骤',
  },
] as const;

/** 根据用户输入推荐工作流名（规则，非 LLM） */
export function suggestWorkflowName(prompt: string): string {
  const lower = prompt.toLowerCase();
  if (/bug|fix|修复|报错|崩溃/.test(lower)) return 'quick-fix';
  if (/计划|plan|拆解/.test(lower)) return 'writing-plans';
  return 'solo-dev';
}
