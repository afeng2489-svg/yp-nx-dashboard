import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Loader2, Send } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { api } from '@/api/client';
import { FACTORY_QUICK_LINES, suggestWorkflowName } from '@/data/factoryQuickStart';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { ActiveExecutionsPanel } from '@/components/dashboard';
import { WorkflowLaunchModal } from '@/components/workflow/WorkflowLaunchModal';
import { cn } from '@/lib/utils';
import type { Workflow } from '@/stores/workflowStore';

interface FactoryConsoleTabProps {
  onRunStarted?: () => void;
}

export function FactoryConsoleTab({ onRunStarted }: FactoryConsoleTabProps) {
  const navigate = useNavigate();
  const { workflows, fetchWorkflows } = useWorkflowStore();
  const { fetchExecutions, connectWebSocket } = useExecutionStore();
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);

  const [intent, setIntent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [launchWorkflow, setLaunchWorkflow] = useState<Workflow | null>(null);
  const [cliReady, setCliReady] = useState<boolean | null>(null);

  useEffect(() => {
    fetchWorkflows();
    api.getClaudeCliConfig().then((cfg) => {
      setCliReady(cfg.source !== 'none' && !!cfg.path);
    }).catch(() => setCliReady(false));
  }, [fetchWorkflows]);

  const quickCards = FACTORY_QUICK_LINES.map((item) => ({
    ...item,
    workflow: workflows.find((w) => w.name === item.workflowName) ?? null,
  }));

  const runQuickPrompt = async (prompt: string) => {
    if (!prompt.trim() || loading) return;
    if (cliReady === false) {
      setError('未绑定本地 Claude Code CLI，请先在设置中检测/配置路径');
      return;
    }
    if (!currentWorkspace?.root_path) {
      setError('请先在顶栏选择工作区（项目文件夹），Claude CLI 才能在该目录读写代码');
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/quick-run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ prompt: prompt.trim() }),
      });
      const data = await res.json();
      if (data.ok) {
        const execId = data.data?.execution_id as string | undefined;
        setIntent('');
        await fetchExecutions();
        if (execId) connectWebSocket(execId);
        onRunStarted?.();
      } else {
        setError(data.error ?? '启动失败');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : '网络错误');
    } finally {
      setLoading(false);
    }
  };

  const handleQuickLine = (item: (typeof quickCards)[number]) => {
    const preset = 'presetTask' in item ? item.presetTask : undefined;
    if (preset) {
      void runQuickPrompt(preset);
      return;
    }
    if (item.workflow) {
      setLaunchWorkflow(item.workflow);
      return;
    }
    setError(`工作流「${item.workflowName}」未找到，请先在资产库确认已加载`);
  };

  const suggested = intent.trim() ? suggestWorkflowName(intent) : null;

  return (
    <div className="space-y-8">
      {cliReady === false && (
        <div className="rounded-xl border border-amber-500/40 bg-amber-500/5 px-4 py-3 text-sm text-amber-800 dark:text-amber-200">
          工厂台 Run 依赖本地 <strong>Claude Code CLI</strong>（非云端 API）。
          <Link to="/settings/ai" className="ml-2 underline font-medium">前往绑定 CLI</Link>
        </div>
      )}
      {!currentWorkspace?.root_path && (
        <div className="rounded-xl border border-sky-500/30 bg-sky-500/5 px-4 py-3 text-sm text-sky-900 dark:text-sky-200">
          请先在顶栏左侧选择<strong>工作区文件夹</strong>，workflow 才会在正确目录调用 Claude CLI 改代码。
        </div>
      )}
      <section className="rounded-2xl border border-border/60 bg-gradient-to-br from-indigo-500/5 via-purple-500/5 to-pink-500/5 p-6">
        <h2 className="text-lg font-semibold mb-1">一句话启动</h2>
        <p className="text-sm text-muted-foreground mb-4">
          描述你要做的事，系统自动匹配 solo-dev / quick-fix 等工作流
          {suggested && (
            <span className="ml-2 text-primary">→ 推荐: {suggested}</span>
          )}
        </p>
        <div className="flex gap-2">
          <textarea
            className="flex-1 min-h-[88px] rounded-xl border border-border/50 bg-background px-4 py-3 text-sm resize-none focus:outline-none focus:ring-2 focus:ring-primary/20"
            placeholder="例如：给 README 增加快速开始章节 / 修复登录页 500 错误"
            value={intent}
            onChange={(e) => setIntent(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void runQuickPrompt(intent);
              }
            }}
          />
          <button
            type="button"
            disabled={!intent.trim() || loading}
            onClick={() => void runQuickPrompt(intent)}
            className="btn-primary self-end flex items-center gap-2 px-5"
          >
            {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
            启动
          </button>
        </div>
        {error && <p className="mt-2 text-sm text-destructive">{error}</p>}
        <p className="mt-2 text-xs text-muted-foreground">⌘/Ctrl + Enter 快速提交</p>
      </section>

      <section>
        <h3 className="text-sm font-medium text-muted-foreground mb-3">快捷入口</h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          {quickCards.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => handleQuickLine(item)}
                className={cn(
                  'group text-left p-4 rounded-xl border border-border/50 hover:border-primary/30',
                  'hover:shadow-md transition-all bg-card/50',
                )}
              >
                <div
                  className={cn(
                    'w-10 h-10 rounded-lg bg-gradient-to-br flex items-center justify-center mb-3',
                    item.gradient,
                  )}
                >
                  <Icon className="w-5 h-5 text-white" />
                </div>
                <p className="font-medium text-sm">{item.title}</p>
                <p className="text-xs text-muted-foreground mt-1">{item.description}</p>
              </button>
            );
          })}
        </div>
      </section>

      <section>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium">进行中的 Run</h3>
          <button
            type="button"
            className="text-xs text-primary hover:underline"
            onClick={() => navigate('/factory?tab=runs')}
          >
            查看全部
          </button>
        </div>
        <ActiveExecutionsPanel />
      </section>

      {launchWorkflow && (
        <WorkflowLaunchModal
          workflow={launchWorkflow}
          onClose={() => setLaunchWorkflow(null)}
        />
      )}
    </div>
  );
}
