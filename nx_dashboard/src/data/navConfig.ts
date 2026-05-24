/** Shared navigation config for Sidebar and CommandPalette */

export interface NavItem {
  id: string;
  label: string;
  path: string;
  command: string;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    label: '主流程',
    items: [
      { id: 'dashboard', label: '仪表盘', path: '/', command: 'go:dashboard' },
      { id: 'guide', label: '使用指南', path: '/guide', command: 'go:guide' },
      { id: 'workflows', label: '工作流', path: '/workflows', command: 'go:workflows' },
      { id: 'executions', label: '执行记录', path: '/executions', command: 'go:executions' },
      { id: 'sessions', label: '会话', path: '/sessions', command: 'go:sessions' },
      { id: 'canvas', label: '可视化画布', path: '/canvas', command: 'go:canvas' },
      { id: 'sprint-board', label: 'Sprint 看板', path: '/sprint-board', command: 'go:sprint-board' },
    ],
  },
  {
    label: 'AI 团队',
    items: [
      { id: 'teams-v2', label: '团队 CLI', path: '/teams-v2', command: 'go:teams-v2' },
      { id: 'roles', label: '角色', path: '/roles', command: 'go:roles' },
      { id: 'group-chat', label: '群组讨论', path: '/group-chat', command: 'go:group-chat' },
      { id: 'team-sessions', label: '团队会话', path: '/team-sessions', command: 'go:team-sessions' },
      { id: 'processes', label: '进程监测', path: '/processes', command: 'go:processes' },
      { id: 'quick-launch', label: '快速启动', path: '/quick-launch', command: 'go:quick-launch' },
    ],
  },
  {
    label: '资源',
    items: [
      { id: 'projects', label: '项目', path: '/projects', command: 'go:projects' },
      { id: 'templates', label: '模板', path: '/templates', command: 'go:templates' },
      { id: 'skills', label: '技能', path: '/skills', command: 'go:skills' },
      { id: 'wisdom', label: '知识库', path: '/wisdom', command: 'go:wisdom' },
      { id: 'knowledge-base', label: 'RAG 知识库', path: '/knowledge-base', command: 'go:knowledge-base' },
    ],
  },
  {
    label: '工具',
    items: [
      { id: 'terminal', label: '终端', path: '/terminal', command: 'go:terminal' },
      { id: 'browser', label: '浏览器', path: '/browser', command: 'go:browser' },
      { id: 'search', label: '搜索', path: '/search', command: 'go:search' },
      { id: 'ui-design', label: 'UI 设计', path: '/ui-design', command: 'go:ui-design' },
      { id: 'tasks', label: '任务', path: '/tasks', command: 'go:tasks' },
      { id: 'cost', label: '成本', path: '/cost', command: 'go:cost' },
    ],
  },
  {
    label: '系统',
    items: [
      { id: 'ai-settings', label: 'AI 设置', path: '/ai-settings', command: 'go:ai-settings' },
      { id: 'settings', label: '设置', path: '/settings', command: 'go:settings' },
    ],
  },
];

export const ALL_NAV_ITEMS = NAV_GROUPS.flatMap((g) => g.items);

export const NAV_BY_COMMAND = Object.fromEntries(ALL_NAV_ITEMS.map((n) => [n.command, n]));

export const NAV_BY_PATH = Object.fromEntries(ALL_NAV_ITEMS.map((n) => [n.path, n]));

/** Resolve active nav id from pathname (exact match, then prefix). */
export function resolveActiveNavId(pathname: string): string {
  if (NAV_BY_PATH[pathname]) return NAV_BY_PATH[pathname].id;
  if (pathname.startsWith('/preview/')) return 'executions';
  if (pathname.startsWith('/sessions')) return 'sessions';

  const prefixMatch = ALL_NAV_ITEMS.filter((n) => n.path !== '/')
    .sort((a, b) => b.path.length - a.path.length)
    .find((n) => pathname.startsWith(n.path + '/') || pathname === n.path);

  return prefixMatch?.id ?? 'dashboard';
}
