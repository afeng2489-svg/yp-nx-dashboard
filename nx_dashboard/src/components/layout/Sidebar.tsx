import {
  LayoutDashboard,
  GitBranch,
  Play,
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
  Compass,
  LayoutTemplate,
  ChevronDown,
  History,
  Zap,
  Factory,
  Package,
  BarChart3,
} from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useUIStore } from '@/stores/uiStore';
import { useState, useEffect } from 'react';
import { api, type ClaudeCliModelResponse } from '@/api/client';
import { NAV_GROUPS, resolveActiveNavId } from '@/data/navConfig';

const NAV_ICONS: Record<string, React.ElementType> = {
  factory: Factory,
  teams: Users,
  assets: Package,
  ops: BarChart3,
  dashboard: LayoutDashboard,
  guide: Compass,
  workflows: GitBranch,
  executions: Play,
  sessions: MessageSquare,
  canvas: LayoutTemplate,
  'sprint-board': ListTodo,
  'teams-v2': Users,
  roles: Bot,
  'group-chat': MessageSquare,
  'team-sessions': History,
  processes: Activity,
  'quick-launch': Zap,
  projects: FolderPlus,
  templates: FolderOpen,
  skills: Wrench,
  wisdom: Brain,
  'knowledge-base': BookOpen,
  terminal: Monitor,
  browser: Globe,
  search: Search,
  'ui-design': Palette,
  tasks: ListTodo,
  cost: DollarSign,
  'ai-settings': Cpu,
  settings: Settings,
};

interface Tab {
  id: string;
  label: string;
  icon: React.ElementType;
  path: string;
}

interface SidebarNavGroup {
  label: string;
  collapsible?: boolean;
  items: Tab[];
}

const navGroups: SidebarNavGroup[] = NAV_GROUPS.map((group) => ({
  label: group.label,
  collapsible: group.label === '工具',
  items: group.items.map((item) => ({
    id: item.id,
    label: item.label,
    path: item.path,
    icon: NAV_ICONS[item.id] ?? LayoutDashboard,
  })),
}));

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
    return () => {
      cancelled = true;
    };
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
        <p className="text-sm font-medium truncate">
          {cliModel?.primary_model || cliModel?.sonnet_model || 'Unknown'}
        </p>
      </div>
      {cliModel?.base_url && (
        <span
          className="text-[10px] text-muted-foreground truncate max-w-[80px]"
          title={cliModel.base_url}
        >
          Proxy
        </span>
      )}
    </div>
  );
}

function NavItem({
  tab,
  isActive,
  sidebarOpen,
  onClick,
  refined = false,
  studio = false,
}: {
  tab: Tab;
  isActive: boolean;
  sidebarOpen: boolean;
  onClick: () => void;
  refined?: boolean;
  studio?: boolean;
}) {
  const Icon = tab.icon;
  const iconOnly = !sidebarOpen;

  return (
    <button
      onClick={onClick}
      title={iconOnly ? tab.label : undefined}
      className={cn(
        'flex items-center rounded-xl transition-colors duration-200',
        iconOnly ? 'w-10 h-10 mx-auto justify-center shrink-0' : 'w-full gap-3 px-3 py-2.5',
        studio
          ? 'hover:bg-accent/80'
          : refined
            ? 'hover:bg-accent/60'
            : 'hover:scale-[1.02] active:scale-[0.98]',
        isActive
          ? studio
            ? 'bg-accent text-foreground border border-border'
            : refined
              ? 'bg-primary/10 text-primary border border-primary/20'
              : 'bg-gradient-to-r from-indigo-500/10 via-purple-500/10 to-pink-500/10 text-primary border border-primary/20 shadow-sm'
          : studio
            ? 'text-muted-foreground hover:text-foreground'
            : 'hover:bg-accent text-muted-foreground hover:text-foreground',
      )}
    >
      <Icon
        className={cn(
          'w-5 h-5 flex-shrink-0 transition-transform duration-200',
          isActive ? (studio ? 'text-foreground' : 'text-primary') : '',
          !refined && !studio && isActive && 'scale-110',
        )}
      />
      {sidebarOpen && (
        <span
          className={cn(
            'font-medium transition-all duration-200 truncate',
            isActive ? 'text-primary' : '',
          )}
        >
          {tab.label}
        </span>
      )}
      {isActive && sidebarOpen && !refined && (
        <div className="ml-auto w-1.5 h-1.5 rounded-full bg-gradient-to-r from-indigo-500 to-purple-500" />
      )}
    </button>
  );
}

export function Sidebar({
  variant = 'classic',
  rail = false,
  theme = 'default',
}: {
  variant?: 'classic' | 'refined';
  rail?: boolean;
  theme?: 'default' | 'studio';
} = {}) {
  const { sidebarOpen, toggleSidebar, setSidebarOpen } = useUIStore();
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({ 工具: true });
  const isRefined = variant === 'refined';
  const isStudio = theme === 'studio';
  const effectiveOpen = rail ? false : sidebarOpen;

  useEffect(() => {
    if (rail) setSidebarOpen(false);
  }, [rail, setSidebarOpen]);

  const activeTab = resolveActiveNavId(location.pathname);

  return (
    <aside
      className={cn(
        'h-full flex flex-col border-r transition-all duration-300 relative shrink-0',
        rail ? 'w-14' : effectiveOpen ? 'w-64' : 'w-16',
        isStudio
          ? 'bg-background border-border text-foreground'
          : isRefined
            ? 'bg-card border-border/40'
            : 'bg-gradient-to-b from-card to-background',
      )}
    >
      {/* Header */}
      <div
        className={cn(
          'flex items-center h-16 px-4 border-b',
          isStudio ? 'border-border' : 'border-border/50',
          effectiveOpen ? 'justify-between' : 'justify-center',
        )}
      >
        {effectiveOpen ? (
          <div className="flex items-center gap-3">
            <div
              className={cn(
                'w-8 h-8 rounded-lg flex items-center justify-center',
                isRefined || isStudio
                  ? isStudio
                    ? 'bg-foreground text-background'
                    : 'bg-primary text-primary-foreground'
                  : 'bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 shadow-lg shadow-indigo-500/25',
              )}
            >
              <Workflow className="w-4 h-4 text-white" />
            </div>
            <span
              className={cn(
                'font-bold text-lg',
                isRefined || isStudio
                  ? isStudio
                    ? 'text-foreground'
                    : 'text-foreground'
                  : 'bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent',
              )}
            >
              TeamFlow
            </span>
          </div>
        ) : (
          <div
            className={cn(
              'w-8 h-8 rounded-lg flex items-center justify-center',
              isStudio
                ? 'bg-foreground text-background'
                : isRefined
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 shadow-lg shadow-indigo-500/25',
            )}
          >
            <Workflow className={cn('w-4 h-4', isStudio ? 'text-background' : 'text-white')} />
          </div>
        )}
        {!rail && (
          <button
            onClick={toggleSidebar}
            className="p-2 rounded-lg hover:bg-accent transition-all duration-200 hover:scale-105 active:scale-95"
          >
            {effectiveOpen ? (
              <ChevronLeft className="w-4 h-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="w-4 h-4 text-muted-foreground" />
            )}
          </button>
        )}
      </div>

      {/* Navigation */}
      <nav
        className={cn(
          'flex-1 overflow-y-auto',
          rail ? 'px-2 py-3 space-y-1' : 'p-3 space-y-4',
        )}
      >
        {navGroups.map((group) => {
          const isCollapsed = group.collapsible && collapsed[group.label];
          const hasActive = group.items.some((t) => t.id === activeTab);

          return (
            <div key={group.label}>
              {effectiveOpen && (
                <button
                  onClick={() =>
                    group.collapsible &&
                    setCollapsed((prev) => ({ ...prev, [group.label]: !prev[group.label] }))
                  }
                  className={cn(
                    'w-full flex items-center justify-between px-3 py-1 mb-1',
                    group.collapsible ? 'cursor-pointer hover:text-foreground' : 'cursor-default',
                  )}
                >
                  <span
                    className={cn(
                      'text-[11px] font-semibold uppercase tracking-wider',
                      hasActive ? 'text-primary' : 'text-muted-foreground/60',
                    )}
                  >
                    {group.label}
                  </span>
                  {group.collapsible && (
                    <ChevronDown
                      className={cn(
                        'w-3 h-3 text-muted-foreground/60 transition-transform',
                        isCollapsed ? '-rotate-90' : '',
                      )}
                    />
                  )}
                </button>
              )}
              {!isCollapsed && (
                <div className={cn(rail ? 'space-y-1' : 'space-y-0.5')}>
                  {group.items.map((tab) => (
                    <NavItem
                      key={tab.id}
                      tab={tab}
                      isActive={activeTab === tab.id}
                      sidebarOpen={effectiveOpen}
                      refined={isRefined}
                      studio={isStudio}
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
      <div className={cn('p-3 border-t space-y-2', isStudio ? 'border-border' : 'border-border/50')}>
        <div className={cn('flex items-center', effectiveOpen ? 'justify-end' : 'justify-center')}>
          {effectiveOpen ? (
            <CliModelDisplay />
          ) : (
            <div className="p-2 rounded-lg" title="CLI 模型">
              <Cpu className="w-5 h-5 text-primary" />
            </div>
          )}
        </div>
        <div
          className={cn(
            'flex items-center gap-3 px-3 py-2 rounded-xl',
            isStudio
              ? 'bg-muted/60'
              : 'bg-gradient-to-r from-indigo-500/5 to-purple-500/5',
            !effectiveOpen && 'justify-center',
          )}
        >
          <div
            className={cn(
              'w-8 h-8 rounded-full flex items-center justify-center text-white text-sm font-bold',
              isStudio
                ? 'bg-muted-foreground'
                : isRefined
                  ? 'bg-primary'
                  : 'bg-gradient-to-br from-indigo-500 to-purple-500 shadow-lg shadow-indigo-500/25',
            )}
          >
            N
          </div>
          {effectiveOpen && (
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium truncate">TeamFlow</p>
              <p className="text-xs text-muted-foreground truncate">v0.1.0</p>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
