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
} from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useUIStore } from '@/stores/uiStore';
import { useState, useEffect } from 'react';
import { api, type ClaudeCliModelResponse } from '@/api/client';

const tabs = [
  { id: 'dashboard' as const, label: '仪表盘', icon: LayoutDashboard, path: '/' },
  { id: 'workflows' as const, label: '工作流', icon: GitBranch, path: '/workflows' },
  { id: 'canvas' as const, label: '可视化画布', icon: LayoutTemplate, path: '/canvas' },
  { id: 'templates' as const, label: '模板', icon: FolderOpen, path: '/templates' },
  { id: 'executions' as const, label: '执行', icon: Play, path: '/executions' },
  { id: 'terminal' as const, label: '终端', icon: Monitor, path: '/terminal' },
  { id: 'sessions' as const, label: '会话', icon: Terminal, path: '/sessions' },
  { id: 'tasks' as const, label: '任务', icon: ListTodo, path: '/tasks' },
  { id: 'ui-design' as const, label: 'UI 设计', icon: Palette, path: '/ui-design' },
  { id: 'wisdom' as const, label: '知识库', icon: Brain, path: '/wisdom' },
  { id: 'search' as const, label: '搜索', icon: Search, path: '/search' },
  { id: 'skills' as const, label: '技能', icon: Wrench, path: '/skills' },
  { id: 'teams' as const, label: '团队', icon: Users, path: '/teams' },
  { id: 'teams-v2' as const, label: '团队 CLI', icon: Users, path: '/teams-v2' },
  { id: 'roles' as const, label: '角色', icon: Bot, path: '/roles' },
  { id: 'projects' as const, label: '项目', icon: FolderPlus, path: '/projects' },
  { id: 'group-chat' as const, label: '群组讨论', icon: MessageSquare, path: '/group-chat' },
  { id: 'processes' as const, label: '进程监测', icon: Activity, path: '/processes' },
  { id: 'cost' as const, label: '成本', icon: DollarSign, path: '/cost' },
  { id: 'knowledge-base' as const, label: 'RAG 知识库', icon: BookOpen, path: '/knowledge-base' },
  { id: 'browser' as const, label: '浏览器', icon: Globe, path: '/browser' },
  // { id: 'ai-settings' as const, label: 'AI 设置', icon: Bot, path: '/ai-settings' },
  { id: 'settings' as const, label: '设置', icon: Settings, path: '/settings' },
];

// Claude CLI 真实模型展示
function CliModelDisplay() {
  const [cliModel, setCliModel] = useState<ClaudeCliModelResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    let attempt = 0;
    const maxAttempts = 30; // 最多重试 30 次（约 30 秒，等 nx_api 起来）

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
          // nx_api 可能还没起来，等一秒重试
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

  const displayName = cliModel?.sonnet_model || 'Unknown';

  return (
    <div className="w-full flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200 hover:bg-accent">
      <Cpu className="w-4 h-4 text-primary flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-xs text-muted-foreground truncate">CLI 模型</p>
        <p className="text-sm font-medium truncate">{displayName}</p>
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

export function Sidebar() {
  const { sidebarOpen, toggleSidebar } = useUIStore();
  const navigate = useNavigate();
  const location = useLocation();

  const currentPath = location.pathname;
  const activeTab = tabs.find((t) => t.path === currentPath)?.id || 'dashboard';

  return (
    <aside
      className={cn(
        'h-full flex flex-col border-r transition-all duration-300 relative',
        sidebarOpen ? 'w-64' : 'w-16',
        'bg-gradient-to-b from-card to-background',
      )}
    >
      {/* Header */}
      <div
        className={cn(
          'flex items-center h-16 px-4 border-b border-border/50',
          sidebarOpen ? 'justify-between' : 'justify-center',
        )}
      >
        {sidebarOpen && (
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/25">
              <Workflow className="w-4 h-4 text-white" />
            </div>
            <span className="font-bold text-lg bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent">
              YpNextFlow
            </span>
          </div>
        )}
        {!sidebarOpen && (
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/25">
            <Workflow className="w-4 h-4 text-white" />
          </div>
        )}
        <button
          onClick={toggleSidebar}
          className={cn(
            'p-2 rounded-lg hover:bg-accent transition-all duration-200',
            'hover:scale-105 active:scale-95',
          )}
        >
          {sidebarOpen ? (
            <ChevronLeft className="w-4 h-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-3 space-y-1">
        {tabs.map(({ id, label, icon: Icon, path }) => {
          const isActive = activeTab === id;
          return (
            <button
              key={id}
              onClick={() => navigate(path)}
              className={cn(
                'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl transition-all duration-200',
                'hover:scale-[1.02] active:scale-[0.98]',
                isActive
                  ? 'bg-gradient-to-r from-indigo-500/10 via-purple-500/10 to-pink-500/10 text-primary border border-primary/20 shadow-sm'
                  : 'hover:bg-accent text-muted-foreground hover:text-foreground',
              )}
            >
              <Icon
                className={cn(
                  'w-5 h-5 flex-shrink-0 transition-transform duration-200',
                  isActive ? 'text-primary scale-110' : '',
                )}
              />
              {sidebarOpen && (
                <span
                  className={cn(
                    'font-medium transition-all duration-200',
                    isActive ? 'text-primary' : '',
                  )}
                >
                  {label}
                </span>
              )}
              {isActive && sidebarOpen && (
                <div className="ml-auto w-1.5 h-1.5 rounded-full bg-gradient-to-r from-indigo-500 to-purple-500" />
              )}
            </button>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-3 border-t border-border/50 space-y-2">
        {/* CLI 模型展示 */}
        <div className={cn('flex items-center', sidebarOpen ? 'justify-end' : 'justify-center')}>
          {sidebarOpen ? (
            <CliModelDisplay />
          ) : (
            <div className="p-2 rounded-lg" title="CLI 模型">
              <Cpu className="w-5 h-5 text-primary" />
            </div>
          )}
        </div>

        <div
          className={cn(
            'flex items-center gap-3 px-3 py-2 rounded-xl bg-gradient-to-r from-indigo-500/5 to-purple-500/5',
            !sidebarOpen && 'justify-center',
          )}
        >
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
