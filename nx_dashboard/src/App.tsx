import { BrowserRouter, Routes, Route, useLocation, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { lazy, Suspense, useEffect, useState } from 'react';
import { Toaster } from 'sonner';
import { Dashboard } from '@/components/layout';
import { PageTransition, ErrorBoundary } from '@/components/ui';
import { useKeyboardHandler } from '@/lib/keyboard';
import { CommandPalette } from '@/components/command';
import { useVersionCheck } from '@/lib/versionCheck';
import { useExecutionStore } from '@/stores/executionStore';
import { WorkflowPauseModal } from '@/components/execution/WorkflowPauseModal';
import { closeBrowserWebview } from '@/pages/BrowserPage';
import { waitForBackend } from '@/api/backendReady';
import { listen } from '@tauri-apps/api/event';
import './index.css';

// Code splitting for heavy pages
const DashboardPage = lazy(() =>
  import('@/pages/DashboardPage').then((m) => ({ default: m.DashboardPage })),
);
const GuidePage = lazy(() => import('@/pages/GuidePage').then((m) => ({ default: m.GuidePage })));
const WorkflowsPage = lazy(() =>
  import('@/pages/WorkflowsPage').then((m) => ({ default: m.WorkflowsPage })),
);
const ExecutionsPage = lazy(() =>
  import('@/pages/ExecutionsPage').then((m) => ({ default: m.ExecutionsPage })),
);
const TerminalPage = lazy(() =>
  import('@/pages/TerminalPage').then((m) => ({ default: m.TerminalPage })),
);
const EditorPage = lazy(() =>
  import('@/pages/EditorPage').then((m) => ({ default: m.EditorPage })),
);
const SettingsPage = lazy(() =>
  import('@/pages/SettingsPage').then((m) => ({ default: m.SettingsPage })),
);
const AISettingsPage = lazy(() =>
  import('@/pages/AISettingsPage').then((m) => ({ default: m.AISettingsPage })),
);
const TasksPage = lazy(() => import('@/pages/TasksPage').then((m) => ({ default: m.TasksPage })));
const SearchPage = lazy(() =>
  import('@/pages/SearchPage').then((m) => ({ default: m.SearchPage })),
);
const TemplatesPage = lazy(() =>
  import('@/pages/TemplatesPage').then((m) => ({ default: m.TemplatesPage })),
);
const SkillsPage = lazy(() => import('@/pages/SkillsPage').then((m) => ({ default: m.default })));
const TeamsPage = lazy(() => import('@/pages/TeamsPage').then((m) => ({ default: m.TeamsPage })));
const TeamsPageV2 = lazy(() =>
  import('@/pages/TeamsPageV2').then((m) => ({ default: m.TeamsPageV2 })),
);
const RolesPage = lazy(() => import('@/pages/RolesPage').then((m) => ({ default: m.RolesPage })));
const ProjectsPage = lazy(() =>
  import('@/pages/ProjectsPage').then((m) => ({ default: m.ProjectsPage })),
);
const GroupChatPage = lazy(() =>
  import('@/pages/GroupChatPage').then((m) => ({ default: m.GroupChatPage })),
);
const ProcessMonitorPage = lazy(() =>
  import('@/pages/ProcessMonitorPage').then((m) => ({ default: m.default })),
);
const BrowserPage = lazy(() =>
  import('@/pages/BrowserPage').then((m) => ({ default: m.BrowserPage })),
);
const UIDesignPage = lazy(() =>
  import('@/pages/UIDesignPage').then((m) => ({ default: m.UIDesignPage })),
);
const CostPage = lazy(() => import('@/pages/CostPage').then((m) => ({ default: m.CostPage })));
const CanvasPage = lazy(() =>
  import('@/pages/CanvasPage').then((m) => ({ default: m.CanvasPage })),
);
const SprintBoardPage = lazy(() =>
  import('@/pages/SprintBoardPage').then((m) => ({ default: m.SprintBoardPage })),
);
const TeamSessionsPage = lazy(() =>
  import('@/pages/TeamSessionsPage').then((m) => ({ default: m.TeamSessionsPage })),
);

// Loading fallback component
function PageLoadingFallback() {
  return (
    <div className="flex items-center justify-center h-[calc(100vh-200px)]">
      <div className="flex flex-col items-center gap-4">
        <div className="w-10 h-10 border-4 border-indigo-500/30 border-t-indigo-500 rounded-full animate-spin" />
        <p className="text-muted-foreground text-sm">Loading...</p>
      </div>
    </div>
  );
}

// Global floating pause card — shown whenever a workflow is waiting for user input
function GlobalPauseCard() {
  const pendingPause = useExecutionStore((s) => s.pendingPause);
  const resumeExecution = useExecutionStore((s) => s.resumeExecution);
  const dismissPause = useExecutionStore((s) => s.dismissPause);

  if (!pendingPause) return null;

  return (
    <WorkflowPauseModal
      pause={pendingPause}
      onResume={(value) => resumeExecution(pendingPause.execution_id, value)}
      onDismiss={dismissPause}
    />
  );
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 20, // 20 seconds default — overridden per-query where needed
      gcTime: 1000 * 60 * 5, // 5 minutes — keep unused cache
      retry: 1,
      refetchOnWindowFocus: false, // queries with staleTime: 0 already refetch on mount
    },
  },
});

// Wrapper component to handle page transitions
function PageWrapper({ children }: { children: React.ReactNode }) {
  const location = useLocation();

  useEffect(() => {
    if (location.pathname !== '/browser') {
      closeBrowserWebview();
    }
  }, [location.pathname]);

  // Editor page doesn't need transitions (fullscreen)
  if (location.pathname === '/editor') {
    return <>{children}</>;
  }

  return <PageTransition key={location.pathname}>{children}</PageTransition>;
}

function WipPage({ title, description }: { title: string; description?: string }) {
  return (
    <div className="flex flex-col items-center justify-center h-full min-h-[60vh] text-center px-6">
      <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-amber-400/20 to-orange-500/20 border border-amber-500/30 flex items-center justify-center mb-4">
        <span className="text-2xl">🚧</span>
      </div>
      <h1 className="text-2xl font-semibold text-foreground mb-2">{title}</h1>
      <span className="inline-block px-2.5 py-1 mb-4 text-xs font-medium rounded-md bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">
        开发中
      </span>
      <p className="text-sm text-muted-foreground max-w-md leading-relaxed">
        {description || '该功能正在开发中，尚未对接业务链路。请先使用其他模块，我们会尽快上线。'}
      </p>
    </div>
  );
}

function App() {
  // Initialize keyboard shortcuts handler
  useKeyboardHandler();

  // Wait for backend to be ready before rendering
  const [backendReady, setBackendReady] = useState(false);
  const [startupError, setStartupError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    // Listen for backend startup error from Tauri
    listen<string>('nx-api-startup-error', (event) => {
      if (!cancelled) {
        setStartupError(event.payload);
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // Not running in Tauri — ignore
      });

    waitForBackend().then((ready) => {
      if (!cancelled && !ready) {
        setStartupError('后端服务未能在规定时间内启动，请检查日志：%TEMP%\\nx_startup.log');
      }
      if (!cancelled) setBackendReady(ready);
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Version check on startup
  const { updateAvailable, showUpdateDialog } = useVersionCheck();

  useEffect(() => {
    if (updateAvailable) {
      // Show update notification after a short delay to let app load first
      const timer = setTimeout(() => {
        showUpdateDialog();
      }, 2000);
      return () => clearTimeout(timer);
    }
  }, [updateAvailable, showUpdateDialog]);

  if (startupError) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="flex flex-col items-center gap-4 max-w-md text-center">
          <div className="w-12 h-12 rounded-full bg-red-500/10 flex items-center justify-center">
            <span className="text-red-500 text-xl">!</span>
          </div>
          <p className="text-foreground text-sm font-medium">后端服务启动失败</p>
          <p className="text-muted-foreground text-xs leading-relaxed">{startupError}</p>
          <button
            className="mt-2 px-4 py-2 text-sm bg-red-500 text-white rounded-md hover:bg-red-600 transition-colors"
            onClick={() => window.location.reload()}
          >
            重试
          </button>
        </div>
      </div>
    );
  }

  if (!backendReady) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="flex flex-col items-center gap-4">
          <div className="w-10 h-10 border-4 border-red-500/30 border-t-red-500 rounded-full animate-spin" />
          <p className="text-muted-foreground text-sm">正在启动后端服务...</p>
        </div>
      </div>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter
        future={{
          v7_startTransition: true,
          v7_relativeSplatPath: true,
        }}
      >
        <ErrorBoundary>
          <Suspense fallback={<PageLoadingFallback />}>
            <Routes>
              <Route element={<Dashboard />}>
                <Route
                  path="/"
                  element={
                    <PageWrapper>
                      <DashboardPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/guide"
                  element={
                    <PageWrapper>
                      <GuidePage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/workflows"
                  element={
                    <PageWrapper>
                      <WorkflowsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/templates"
                  element={
                    <PageWrapper>
                      <TemplatesPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/executions"
                  element={
                    <PageWrapper>
                      <ExecutionsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/terminal"
                  element={
                    <PageWrapper>
                      <TerminalPage />
                    </PageWrapper>
                  }
                />
                <Route path="/editor" element={<EditorPage />} />
                <Route
                  path="/sessions"
                  element={
                    <PageWrapper>
                      <WipPage
                        title="会话"
                        description="会话模块尚未接入工作流执行链路，暂不可用。请使用「执行记录」查看工作流运行情况。"
                      />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/tasks"
                  element={
                    <PageWrapper>
                      <TasksPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/wisdom"
                  element={
                    <PageWrapper>
                      <WipPage
                        title="知识库"
                        description="团队经验沉淀的知识库功能正在开发中，尚未接入 Agent 执行链路。敬请期待。"
                      />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/search"
                  element={
                    <PageWrapper>
                      <SearchPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/skills"
                  element={
                    <PageWrapper>
                      <SkillsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/settings"
                  element={
                    <PageWrapper>
                      <SettingsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/ai-settings"
                  element={
                    <PageWrapper>
                      <AISettingsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/teams"
                  element={
                    <PageWrapper>
                      <TeamsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/teams-v2"
                  element={
                    <PageWrapper>
                      <TeamsPageV2 />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/roles"
                  element={
                    <PageWrapper>
                      <RolesPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/projects"
                  element={
                    <PageWrapper>
                      <ProjectsPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/group-chat"
                  element={
                    <PageWrapper>
                      <GroupChatPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/processes"
                  element={
                    <PageWrapper>
                      <ProcessMonitorPage />
                    </PageWrapper>
                  }
                />
                <Route path="/browser" element={<BrowserPage />} />
                <Route
                  path="/cost"
                  element={
                    <PageWrapper>
                      <CostPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/knowledge-base"
                  element={
                    <PageWrapper>
                      <WipPage
                        title="RAG 知识库"
                        description="基于文档向量检索的 RAG 知识库正在开发中，仅 workflow stage 的 rag 字段可手动配置。敬请期待完整 UI。"
                      />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/ui-design"
                  element={
                    <PageWrapper>
                      <UIDesignPage />
                    </PageWrapper>
                  }
                />
                <Route path="/canvas" element={<CanvasPage />} />
                <Route
                  path="/sprint-board"
                  element={
                    <PageWrapper>
                      <SprintBoardPage />
                    </PageWrapper>
                  }
                />
                <Route
                  path="/team-sessions"
                  element={
                    <PageWrapper>
                      <TeamSessionsPage />
                    </PageWrapper>
                  }
                />
                {/* 404 — unknown routes redirect to home */}
                <Route path="*" element={<Navigate to="/" replace />} />
              </Route>
            </Routes>
          </Suspense>
        </ErrorBoundary>

        {/* Command Palette — inside BrowserRouter for useNavigate */}
        <CommandPalette />
      </BrowserRouter>

      {/* Toast notifications */}
      <Toaster
        position="bottom-right"
        toastOptions={{
          style: {
            background: 'hsl(var(--card))',
            border: '1px solid hsl(var(--border))',
            color: 'hsl(var(--foreground))',
          },
        }}
      />

      {/* Global workflow pause card — bottom-right floating */}
      <GlobalPauseCard />
    </QueryClientProvider>
  );
}

export default App;
