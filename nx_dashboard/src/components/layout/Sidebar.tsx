import {
  LayoutDashboard,
  GitBranch,
  Play,
  Terminal,
  Settings,
  ChevronLeft,
  ChevronRight,
  Monitor,
  Workflow,
  Search,
  ListTodo,
  Brain,
  FolderOpen,
  Wrench,
  Bot,
  Users,
  FolderPlus,
  Loader2,
  MessageSquare,
  Activity,
  Globe,
  Palette,
  Cpu,
  DollarSign,
  BookOpen,
  LayoutTemplate,
  ChevronDown,
} from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useUIStore } from '@/stores/uiStore';
import { useState, useEffect } from 'react';
import { api, type ClaudeCliModelResponse } from '@/api/client';

type TabId =
  | 'dashboard' | 'workflows' | 'canvas' | 'executions' | 'sprint-board'
  | 'teams' | 'teams-v2' | 'roles' | 'group-chat' | 'sessions' | 'processes'
  | 'projects' | 'templates' | 'skills' | 'wisdom' | 'knowledge-base'
  | 'terminal' | 'browser' | 'search' | 'ui-design' | 'tasks' | 'cost'
  | 'settings';

interface Tab {
  id: TabId;
  label: string;
  icon: React.ElementType;
  path: string;
}

interface NavGroup {
  label: string;
  collapsible?: boolean;
  items: Tab[];
}

const navGroups: NavGroup[] = [
  {
    label: '主流程',
    items: [
      { id: 'dashboard', label: '仪表盘', icon: LayoutDashboard, path: '/' },
      { id: 'workflows', label: '工作流', icon: GitBranch, path: '/workflows' },
      { id: 'executions', label: '执行记录', icon: Play, path: '/executions' },
      { id: 'canvas', label: '可视化画布', icon: LayoutTemplate, path: '/canvas' },
      { id: 'sprint-board', label: 'Sprint 看板', icon: ListTodo, path: '/sprint-board' },
    ],
  },
  {
    label: 'AI 团队',
    items: [
      { id: 'teams', label: '团队', icon: Users, path: '/teams' },
      { id: 'roles', label: '角色', icon: Bot, path: '/roles' },
      { id: 'group-chat', label: '群组讨论', icon: MessageSquare, path: '/group-chat' },
      { id: 'sessions', label: '会话', icon: Terminal, path: '/sessions' },
      { id: 'processes', label: '进程监测', icon: Activity, path: '/processes' },
      { id: 'teams-v2', label: '团队 CLI', icon: Users, path: '/teams-v2' },
    ],
  },
  {
    label: '资源',
    items: [
      { id: 'projects', label: '项目', icon: FolderPlus, path: '/projects' },
      { id: 'templates', label: '模板', icon: FolderOpen, path: '/templates' },
      { id: 'skills', label: '技能', icon: Wrench, path: '/skills' },
      { id: 'wisdom', label: '知识库', icon: Brain, path: '/wisdom' },
      { id: 'knowledge-base', label: 'RAG 知识库', icon: BookOpen, path: '/knowledge-base' },
    ],
  },
  {
    label: '工具',
    collapsible: true,
    items: [
      { id: 'terminal', label: '终端', icon: Monitor, path: '/terminal' },
      { id: 'browser', label: '浏览器', icon: Globe, path: '/browser' },
      { id: 'search', label: '搜索', icon: Search, path: '/search' },
      { id: 'ui-design', label: 'UI 设计', icon: Palette, path: '/ui-design' },
      { id: 'tasks', label: '任务', icon: ListTodo, path: '/tasks' },
      { id: 'cost', label: '成本', icon: DollarSign, path: '/cost' },
    ],
  },
  {
    label: '系统',
    items: [
      { id: 'settings', label: '设置', icon: Settings, path: '/settings' },
    ],
  },
];

function CliModelDisplay() {
  const [cliModel, setCliModel] = useState<ClaudeCliModelResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    let attempt = 0;
    const maxAttempts = 30;

    const tryFetch = async () => {
      while (!cancelled && attempt < maxAttempts) {
        attempt += 1;
        try {
          const data = await api.getClaudeCliModel();
          if (!cancelled) {
            setCliModel(data);
            setLoading(false);
          }
          return;
        } catch {
          await new Promise((r) => setTimeout(r, 1000));
        }
      }
      if (!cancelled) setLoading(false);
    };

    void tryFetch();
    return () => { cancelled = true; };
  }, []);

  if (loading) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground">
        <Loader2 className="w-4 h-4 animate-spin" />
        <span className="text-xs">检测模型...</span>
      </div>
    );
  }

  return (
    <div className="w-full flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200 hover:bg-accent">
      <Cpu className="w-4 h-4 text-primary flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-xs text-muted-foreground truncate">CLI 模型</p>
        <p className="text-sm font-medium truncate">{cliModel?.sonnet_model || 'Unknown'}</p>
      </div>
      {cliModel?.base_url && (
        <span className="text-[10px] text-muted-foreground truncate max-w-[80px]" title={cliModel.base_url}>
          Proxy
        </span>
      )}
    </div>
  );
}

function NavItem({ tab, isActive, sidebarOpen, onClick }: {
  tab: Tab;
  isActive: boolean;
  sidebarOpen: boolean;
  onClick: () => void;
}) {
  const Icon = tab.icon;
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl transition-all duration-200',
        'hover:scale-[1.02] active:scale-[0.98]',
        isActive
          ? 'bg-gradient-to-r from-indigo-500/10 via-purple-500/10 to-pink-500/10 text-primary border border-primary/20 shadow-sm'
          : 'hover:bg-accent text-muted-foreground hover:text-foreground',
      )}
    >
      <Icon className={cn('w-5 h-5 flex-shrink-0 transition-transform duration-200', isActive ? 'text-primary scale-110' : '')} />
      {sidebarOpen && (
        <span className={cn('font-medium transition-all duration-200', isActive ? 'text-primary' : '')}>
          {tab.label}
        </span>
      )}
      {isActive && sidebarOpen && (
        <div className="ml-auto w-1.5 h-1.5 rounded-full bg-gradient-to-r from-indigo-500 to-purple-500" />
      )}
    </button>
  );
}

export function Sidebar() {
  const { sidebarOpen, toggleSidebar } = useUIStore();
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({ '工具': true });

  const activeTab = navGroups
    .flatMap((g) => g.items)
    .find((t) => t.path === location.pathname)?.id || 'dashboard';

  return (
    <aside
      className={cn(
        'h-full flex flex-col border-r transition-all duration-300 relative',
        sidebarOpen ? 'w-64' : 'w-16',
        'bg-gradient-to-b from-card to-background',
      )}
    >
      {/* Header */}
      <div className={cn('flex items-center h-16 px-4 border-b border-border/50', sidebarOpen ? 'justify-between' : 'justify-center')}>
        {sidebarOpen ? (
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/25">
              <Workflow className="w-4 h-4 text-white" />
            </div>
            <span className="font-bold text-lg bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent">
              YpNextFlow
            </span>
          </div>
        ) : (
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/25">
            <Workflow className="w-4 h-4 text-white" />
          </div>
        )}
        <button
          onClick={toggleSidebar}
          className="p-2 rounded-lg hover:bg-accent transition-all duration-200 hover:scale-105 active:scale-95"
        >
          {sidebarOpen ? <ChevronLeft className="w-4 h-4 text-muted-foreground" /> : <ChevronRight className="w-4 h-4 text-muted-foreground" />}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-3 space-y-4 overflow-y-auto">
        {navGroups.map((group) => {
          const isCollapsed = group.collapsible && collapsed[group.label];
          const hasActive = group.items.some((t) => t.id === activeTab);

          return (
            <div key={group.label}>
              {sidebarOpen && (
                <button
                  onClick={() => group.collapsible && setCollapsed((prev) => ({ ...prev, [group.label]: !prev[group.label] }))}
                  className={cn(
                    'w-full flex items-center justify-between px-3 py-1 mb-1',
                    group.collapsible ? 'cursor-pointer hover:text-foreground' : 'cursor-default',
                  )}
                >
                  <span className={cn('text-[11px] font-semibold uppercase tracking-wider', hasActive ? 'text-primary' : 'text-muted-foreground/60')}>
                    {group.label}
                  </span>
                  {group.collapsible && (
                    <ChevronDown className={cn('w-3 h-3 text-muted-foreground/60 transition-transform', isCollapsed ? '-rotate-90' : '')} />
                  )}
                </button>
              )}
              {!isCollapsed && (
                <div className="space-y-0.5">
                  {group.items.map((tab) => (
                    <NavItem
                      key={tab.id}
                      tab={tab}
                      isActive={activeTab === tab.id}
                      sidebarOpen={sidebarOpen}
                      onClick={() => navigate(tab.path)}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-3 border-t border-border/50 space-y-2">
        <div className={cn('flex items-center', sidebarOpen ? 'justify-end' : 'justify-center')}>
          {sidebarOpen ? (
            <CliModelDisplay />
          ) : (
            <div className="p-2 rounded-lg" title="CLI 模型">
              <Cpu className="w-5 h-5 text-primary" />
            </div>
          )}
        </div>
        <div className={cn('flex items-center gap-3 px-3 py-2 rounded-xl bg-gradient-to-r from-indigo-500/5 to-purple-500/5', !sidebarOpen && 'justify-center')}>
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center text-white text-sm font-bold shadow-lg shadow-indigo-500/25">
            N
          </div>
          {sidebarOpen && (
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium truncate">YpNextFlow</p>
              <p className="text-xs text-muted-foreground truncate">v0.1.0</p>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
