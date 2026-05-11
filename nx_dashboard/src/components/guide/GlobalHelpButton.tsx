import { useLocation } from 'react-router-dom';
import { PageHelpButton } from './PageHelpButton';

// path → moduleId
const PATH_TO_MODULE: Record<string, string> = {
  '/': 'dashboard',
  '/guide': 'dashboard',
  '/workflows': 'workflows',
  '/executions': 'executions',
  '/canvas': 'canvas',
  '/sprint-board': 'sprint-board',
  '/teams': 'teams',
  '/teams-v2': 'teams-v2',
  '/roles': 'roles',
  '/group-chat': 'group-chat',
  '/processes': 'processes',
  '/projects': 'projects',
  '/templates': 'templates',
  '/skills': 'skills',
  '/terminal': 'terminal',
  '/browser': 'browser',
  '/search': 'search',
  '/ui-design': 'ui-design',
  '/tasks': 'tasks',
  '/cost': 'cost',
  '/settings': 'settings',
  '/ai-settings': 'ai-settings',
  '/editor': 'editor',
};

const HIDE_ON: string[] = ['/guide', '/editor'];

export function GlobalHelpButton() {
  const { pathname } = useLocation();
  if (HIDE_ON.includes(pathname)) return null;

  const moduleId = PATH_TO_MODULE[pathname] ?? inferFromPrefix(pathname);
  if (!moduleId) return null;

  return <PageHelpButton moduleId={moduleId} floating />;
}

function inferFromPrefix(path: string): string | null {
  const match = Object.keys(PATH_TO_MODULE).find((p) => p !== '/' && path.startsWith(p + '/'));
  return match ? PATH_TO_MODULE[match] : null;
}
