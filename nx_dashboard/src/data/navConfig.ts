/** AF-01 工厂台 — 5 项主导航 */

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
    label: 'AI 工厂',
    items: [
      { id: 'factory', label: '工厂台', path: '/factory', command: 'go:factory' },
      { id: 'teams', label: '团队', path: '/teams', command: 'go:teams' },
      { id: 'assets', label: '资产库', path: '/assets', command: 'go:assets' },
      { id: 'ops', label: '运营', path: '/ops', command: 'go:ops' },
      { id: 'settings', label: '设置', path: '/settings', command: 'go:settings' },
    ],
  },
];

export const ALL_NAV_ITEMS = NAV_GROUPS.flatMap((g) => g.items);

export const NAV_BY_COMMAND = Object.fromEntries(ALL_NAV_ITEMS.map((n) => [n.command, n]));

export const NAV_BY_PATH = Object.fromEntries(ALL_NAV_ITEMS.map((n) => [n.path, n]));

/** 旧路由 → 新路由（AF-01 redirect） */
export const LEGACY_PATH_REDIRECTS: Record<string, string> = {
  '/': '/factory',
  '/dashboard': '/factory',
  '/guide': '/factory',
  '/workflows': '/assets?tab=workflows',
  '/executions': '/factory?tab=runs',
  '/canvas': '/factory?tab=runs',
  '/quick-launch': '/factory',
  '/teams-v2': '/teams',
  '/group-chat': '/teams?tab=discuss',
  '/roles': '/assets?tab=roles',
  '/skills': '/assets?tab=skills',
  '/templates': '/assets?tab=workflows',
  '/wisdom': '/assets?tab=knowledge',
  '/knowledge-base': '/assets?tab=knowledge',
  '/cost': '/ops?tab=cost',
  '/team-sessions': '/ops?tab=history',
  '/processes': '/ops?tab=processes',
  '/sprint-board': '/ops?tab=sprint',
  '/ai-settings': '/settings/ai',
  '/projects': '/settings/projects',
};

export function resolveActiveNavId(pathname: string): string {
  if (pathname.startsWith('/factory')) return 'factory';
  if (pathname.startsWith('/teams')) return 'teams';
  if (pathname.startsWith('/assets')) return 'assets';
  if (pathname.startsWith('/ops')) return 'ops';
  if (pathname.startsWith('/settings')) return 'settings';
  if (pathname.startsWith('/preview/')) return 'factory';

  const legacy = Object.keys(LEGACY_PATH_REDIRECTS)
    .filter((p) => p !== '/')
    .sort((a, b) => b.length - a.length)
    .find((p) => pathname === p || pathname.startsWith(p + '/'));
  if (legacy) {
    const target = LEGACY_PATH_REDIRECTS[legacy];
    if (target.startsWith('/factory')) return 'factory';
    if (target.startsWith('/teams')) return 'teams';
    if (target.startsWith('/assets')) return 'assets';
    if (target.startsWith('/ops')) return 'ops';
  }

  return 'factory';
}
