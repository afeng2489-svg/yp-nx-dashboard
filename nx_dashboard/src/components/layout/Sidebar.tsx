import { LayoutDashboard, GitBranch, Play, Terminal, Settings, ChevronLeft, ChevronRight, Monitor, Workflow, Search, ListTodo, Brain, FolderOpen, Wrench, Bot, Users, FolderPlus, ChevronDown, Loader2, MessageSquare, Activity, Globe, Palette } from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useUIStore } from '@/stores/uiStore';
import { useAIConfigStore } from '@/stores/aiConfigStore';
import { useState, useRef, useEffect } from 'react';

const tabs = [
  { id: 'dashboard' as const, label: '仪表盘', icon: LayoutDashboard, path: '/' },
  { id: 'workflows' as const, label: '工作流', icon: GitBranch, path: '/workflows' },
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
  { id: 'roles' as const, label: '角色', icon: Bot, path: '/roles' },
  { id: 'projects' as const, label: '项目', icon: FolderPlus, path: '/projects' },
  { id: 'group-chat' as const, label: '群组讨论', icon: MessageSquare, path: '/group-chat' },
  { id: 'processes' as const, label: '进程监测', icon: Activity, path: '/processes' },
  { id: 'browser' as const, label: '浏览器', icon: Globe, path: '/browser' },
  { id: 'ai-settings' as const, label: 'AI 设置', icon: Bot, path: '/ai-settings' },
  { id: 'settings' as const, label: '设置', icon: Settings, path: '/settings' },
];

// Quick Model Switcher Component
function QuickModelSwitcher() {
  const { models, selectedModel, modelsLoading, fetchModels, fetchSelectedModel, setSelectedModel } = useAIConfigStore();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchModels();
    fetchSelectedModel();
  }, [fetchModels, fetchSelectedModel]);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleModelSelect = async (modelId: string) => {
    await setSelectedModel(modelId);
    setIsOpen(false);
  };

  // Group models by provider
  const modelsByProvider = models.reduce((acc, model) => {
    if (!acc[model.provider]) {
      acc[model.provider] = [];
    }
    acc[model.provider].push(model);
    return acc;
  }, {} as Record<string, typeof models>);

  const providerNames: Record<string, string> = {
    'anthropic': 'Claude',
    'openai': 'OpenAI',
    'google': 'Google',
    'ollama': 'Ollama',
    'deepseek': 'DeepSeek',
  };

  if (modelsLoading && models.length === 0) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground">
        <Loader2 className="w-4 h-4 animate-spin" />
        <span className="text-xs">加载模型...</span>
      </div>
    );
  }

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'w-full flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200',
          'hover:bg-accent text-left'
        )}
      >
        <Bot className="w-4 h-4 text-primary flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <p className="text-xs text-muted-foreground truncate">当前模型</p>
          <p className="text-sm font-medium truncate">
            {selectedModel?.display_name || '未选择'}
          </p>
        </div>
        <ChevronDown className={cn('w-4 h-4 transition-transform', isOpen && 'rotate-180')} />
      </button>

      {isOpen && (
        <div className="absolute bottom-full left-0 right-0 mb-1 bg-card rounded-lg border border-border shadow-lg overflow-hidden z-50 max-h-64 overflow-y-auto">
          <div className="p-2 space-y-2">
            {Object.entries(modelsByProvider).map(([provider, providerModels]) => (
              <div key={provider}>
                <p className="text-xs text-muted-foreground px-2 py-1">
                  {providerNames[provider] || provider}
                </p>
                {providerModels.map((model) => {
                  const isSelected = selectedModel?.model_id === model.model_id;
                  return (
                    <button
                      key={model.model_id}
                      onClick={() => handleModelSelect(model.model_id)}
                      className={cn(
                        'w-full text-left px-2 py-1.5 rounded text-sm transition-colors',
                        isSelected
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'hover:bg-accent'
                      )}
                    >
                      <div className="flex items-center justify-between">
                        <span className="truncate">{model.display_name}</span>
                        {isSelected && <Bot className="w-3 h-3" />}
                      </div>
                    </button>
                  );
                })}
              </div>
            ))}
            {models.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-2">
                暂无可用模型
              </p>
            )}
          </div>
        </div>
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
        'bg-gradient-to-b from-card to-background'
      )}
    >
      {/* Header */}
      <div className={cn(
        'flex items-center h-16 px-4 border-b border-border/50',
        sidebarOpen ? 'justify-between' : 'justify-center'
      )}>
        {sidebarOpen && (
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/25">
              <Workflow className="w-4 h-4 text-white" />
            </div>
            <span className="font-bold text-lg bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent">
              NexusFlow
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
            'hover:scale-105 active:scale-95'
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
                  : 'hover:bg-accent text-muted-foreground hover:text-foreground'
              )}
            >
              <Icon className={cn(
                'w-5 h-5 flex-shrink-0 transition-transform duration-200',
                isActive ? 'text-primary scale-110' : ''
              )} />
              {sidebarOpen && (
                <span className={cn(
                  'font-medium transition-all duration-200',
                  isActive ? 'text-primary' : ''
                )}>
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
        {/* Quick Model Switcher */}
        <div className={cn(
          'flex items-center',
          sidebarOpen ? 'justify-end' : 'justify-center'
        )}>
          {sidebarOpen ? (
            <QuickModelSwitcher />
          ) : (
            <button
              onClick={() => navigate('/ai-settings')}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
              title="快速切换模型"
            >
              <Bot className="w-5 h-5 text-primary" />
            </button>
          )}
        </div>

        <div className={cn(
          'flex items-center gap-3 px-3 py-2 rounded-xl bg-gradient-to-r from-indigo-500/5 to-purple-500/5',
          !sidebarOpen && 'justify-center'
        )}>
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center text-white text-sm font-bold shadow-lg shadow-indigo-500/25">
            N
          </div>
          {sidebarOpen && (
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium truncate">NexusFlow</p>
              <p className="text-xs text-muted-foreground truncate">v0.1.0</p>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
