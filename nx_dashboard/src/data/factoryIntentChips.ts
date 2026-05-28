import { FolderPlus, Rocket, Sparkles } from 'lucide-react';
import { GOLDEN_PATH_TASK } from '@/services/factoryMetrics';

/** 引导模式首屏 3 chip — 用户语言，内部映射 workflow */
export const FACTORY_INTENT_CHIPS = [
  {
    id: 'new-project',
    label: '新建项目',
    description: '从想法到可运行骨架（7 阶段产线）',
    icon: FolderPlus,
    workflowName: 'greenfield-mvp',
    placeholder: '例如：做一个本地优先的 Todo App',
  },
  {
    id: 'change-code',
    label: '改代码',
    description: '在现有仓库上加功能或改 README',
    icon: Rocket,
    workflowName: 'solo-dev',
    placeholder: '例如：给登录页加验证码 / 更新 README 快速开始',
  },
  {
    id: 'demo',
    label: '试用演示',
    description: 'Golden Path · 约 2 分钟看到产线',
    icon: Sparkles,
    workflowName: 'solo-dev',
    presetTask: GOLDEN_PATH_TASK,
  },
] as const;

export type FactoryIntentChipId = (typeof FACTORY_INTENT_CHIPS)[number]['id'];
