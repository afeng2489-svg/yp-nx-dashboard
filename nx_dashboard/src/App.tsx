import { BrowserRouter, Routes, Route, useLocation, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { lazy, Suspense, useEffect } from 'react';
import { Toaster } from 'sonner';
import { Dashboard } from '@/components/layout';
import { PageTransition } from '@/components/ui';
import { useKeyboardHandler } from '@/lib/keyboard';
import { CommandPalette } from '@/components/command';
import { useVersionCheck } from '@/lib/versionCheck';
import { useExecutionStore } from '@/stores/executionStore';
import { WorkflowPauseModal } from '@/components/execution/WorkflowPauseModal';
import './index.css';

// Code splitting for heavy pages
const DashboardPage = lazy(() =>
  import('@/pages/DashboardPage').then((m) => ({ default: m.DashboardPage })),
);
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
const SessionsPage = lazy(() =>
  import('@/pages/SessionsPage').then((m) => ({ default: m.SessionsPage })),
);
const SettingsPage = lazy(() =>
  import('@/pages/SettingsPage').then((m) => ({ default: m.SettingsPage })),
);
const AISettingsPage = lazy(() =>
  import('@/pages/AISettingsPage').then((m) => ({ default: m.AISettingsPage })),
);
const WisdomPage = lazy(() =>
  import('@/pages/WisdomPage').then((m) => ({ default: m.WisdomPage })),
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

  // Editor page doesn't need transitions (fullscreen)
  if (location.pathname === '/editor') {
    return <>{children}</>;
  }

  return <PageTransition key={location.pathname}>{children}</PageTransition>;
}

function App() {
  // Initialize keyboard shortcuts handler
  useKeyboardHandler();

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

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
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
                    <SessionsPage />
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
                    <WisdomPage />
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
                path="/ui-design"
                element={
                  <PageWrapper>
                    <UIDesignPage />
                  </PageWrapper>
                }
              />
              {/* 404 — unknown routes redirect to home */}
              <Route path="*" element={<Navigate to="/" replace />} />
            </Route>
          </Routes>
        </Suspense>

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
