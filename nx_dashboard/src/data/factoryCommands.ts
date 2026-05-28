/** AF-10 ⌘K 工厂超级命令 */
export interface FactoryCommand {
  command: string;
  description: string;
  path?: string;
}

export const FACTORY_GO_COMMANDS: FactoryCommand[] = [
  { command: 'go:factory:console', description: '工厂台 · 控制台', path: '/factory' },
  { command: 'go:factory:runs', description: '工厂台 · 运行中', path: '/factory?tab=runs' },
  { command: 'go:factory:approvals', description: '工厂台 · 待审批', path: '/factory?tab=approvals' },
  {
    command: 'go:factory:deliverables',
    description: '工厂台 · 产物',
    path: '/factory?tab=deliverables',
  },
];

export const FACTORY_ACTION_COMMANDS: FactoryCommand[] = [
  { command: 'factory:run', description: '快速启动 Run（factory:run <prompt>）' },
  { command: 'factory:approve', description: '批准第一个待审批项' },
  { command: 'factory:reject', description: '驳回第一个待审批项' },
  { command: 'project:switch', description: '切换项目（project:switch <name>）' },
  { command: 'team:switch', description: '切换团队（team:switch <name>）' },
  { command: 'run:open', description: '打开 Run 上下文（run:open <id>）' },
  { command: 'terminal:open', description: '打开终端面板（工作室）/ 工具抽屉（引导）' },
];

export const LAYOUT_COMMANDS: FactoryCommand[] = [
  { command: 'layout:guided', description: '布局 · 引导模式（侧栏 + Tab）' },
  { command: 'layout:studio', description: '布局 · 工作室模式（文件树 + 终端）' },
  { command: 'layout:classic', description: '界面 · 经典风格（渐变 / 完整卡片）' },
  { command: 'layout:refined', description: '界面 · 精简风格（贴近设计稿）' },
];

export const ALL_FACTORY_COMMANDS = [
  ...FACTORY_GO_COMMANDS,
  ...FACTORY_ACTION_COMMANDS,
  ...LAYOUT_COMMANDS,
];

export const FACTORY_COMMAND_PATHS = Object.fromEntries(
  FACTORY_GO_COMMANDS.filter((c) => c.path).map((c) => [c.command, c.path!]),
);
