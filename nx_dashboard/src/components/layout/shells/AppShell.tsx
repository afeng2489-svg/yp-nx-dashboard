import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { PanelLeft, PanelLeftClose, Terminal } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  ShellThemeProvider,
  useResolvedShellTheme,
} from '@/components/layout/ShellThemeContext';
import { useThemeStore } from '@/stores/themeStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { normalizeLayoutMode } from '@/data/layoutModes';
import { DEFAULT_LAYOUT_VARIANT, type LayoutVariant } from '@/data/layoutVariants';
import { Sidebar } from '@/components/layout/Sidebar';
import { WorkspaceSelector } from '@/components/workspace/WorkspaceSelector';
import { FactoryGlobalBar } from '@/components/factory/FactoryGlobalBar';
import { FactoryStatusBar } from '@/components/factory/FactoryStatusBar';
import { FileSidebar } from '@/components/explorer/FileSidebar';
import { ClaudeCliMissingBanner } from '@/components/global/ClaudeCliMissingBanner';
import { LayoutModeMenu } from '@/components/layout/LayoutModeMenu';
import { LayoutVariantMenu } from '@/components/layout/LayoutVariantMenu';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import { useShrinkBelow } from '@/hooks/useResponsive';
import { useUIStore } from '@/stores/uiStore';
import { DashboardMainArea } from './DashboardMainArea';
import { DashboardOverlays } from './DashboardOverlays';

function ShellHeader({
  variant,
  isStudio,
  showFileToggle,
  showFileSidebar,
  onToggleFileSidebar,
}: {
  variant: LayoutVariant;
  isStudio: boolean;
  showFileToggle?: boolean;
  showFileSidebar: boolean;
  onToggleFileSidebar: () => void;
}) {
  const isRefined = variant === 'refined';

  if (isRefined) {
    return (
      <header className="relative z-50 min-h-12 px-4 sm:px-6 py-2 flex items-center justify-between gap-2 border-b shrink-0 border-border bg-background text-foreground">
        <div className="flex items-center gap-2 sm:gap-3 min-w-0 flex-1">
          <WorkspaceSelector />
          <div className="hidden md:block h-5 w-px bg-border/60" />
          <FactoryGlobalBar />
          {showFileToggle && (
            <button
              type="button"
              onClick={onToggleFileSidebar}
              className={cn(
                'p-2 rounded-md transition-colors flex-shrink-0 hover:bg-accent',
                showFileSidebar && 'bg-accent',
              )}
              title={showFileSidebar ? '隐藏文件浏览器' : '显示文件浏览器'}
            >
              {showFileSidebar ? (
                <PanelLeftClose className="w-4 h-4" />
              ) : (
                <PanelLeft className="w-4 h-4" />
              )}
            </button>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <LayoutVariantMenu compact />
          <LayoutModeMenu compact />
        </div>
      </header>
    );
  }

  return (
    <header className="relative z-50 min-h-14 px-4 sm:px-6 py-2 flex items-center justify-between gap-2 border-b border-border/50 bg-card flex-wrap shrink-0">
      <div className="flex items-center gap-2 sm:gap-4 min-w-0 flex-1">
        <WorkspaceSelector />
        <div className="hidden md:block h-6 w-px bg-border/60" />
        <FactoryGlobalBar />
        {showFileToggle && (
          <button
            type="button"
            onClick={onToggleFileSidebar}
            className={cn(
              'p-2 rounded-lg hover:bg-accent transition-colors flex-shrink-0',
              showFileSidebar && 'bg-accent',
            )}
            title={showFileSidebar ? '隐藏文件浏览器' : '显示文件浏览器'}
          >
            {showFileSidebar ? (
              <PanelLeftClose className="w-4 h-4" />
            ) : (
              <PanelLeft className="w-4 h-4" />
            )}
          </button>
        )}
      </div>
      <div className="flex items-center gap-1 shrink-0">
        <LayoutVariantMenu />
        <LayoutModeMenu />
      </div>
    </header>
  );
}

function StudioRefinedStatusBar() {
  const isDark = useThemeStore((s) => s.resolvedTheme === 'dark');
  const toggleIntegrated = useFactoryDrawerStore((s) => s.toggleIntegrated);
  const integratedVisible = useFactoryDrawerStore((s) => s.integratedVisible);

  return (
    <div
      className={cn(
        'h-7 px-3 flex items-center justify-between text-[10px] font-mono shrink-0 border-t',
        isDark
          ? 'text-muted-foreground border-border bg-background'
          : 'text-muted-foreground border-border/40 bg-muted/30',
      )}
    >
      <span>Studio</span>
      <span className="flex items-center gap-2">
        <button
          type="button"
          onClick={toggleIntegrated}
          className={cn(
            'inline-flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-accent/50',
            integratedVisible ? 'text-foreground' : 'text-muted-foreground',
          )}
          title="切换终端面板 (⌃`)"
        >
          <Terminal className="w-3 h-3" />
          终端
        </button>
        <span className="hidden sm:inline text-muted-foreground">⌃` 切换 · ⌘K 命令</span>
      </span>
    </div>
  );
}

function useStudioRefinedBoot(enabled: boolean) {
  const location = useLocation();
  useEffect(() => {
    if (!enabled) return;
    if (!location.pathname.startsWith('/factory')) return;
    const key = 'studio-refined-terminal-boot';
    if (sessionStorage.getItem(key)) return;
    sessionStorage.setItem(key, '1');
    useFactoryDrawerStore.getState().showIntegrated();
  }, [enabled, location.pathname]);
}

function FileSidebarPanel({
  variant,
  isStudio,
}: {
  variant: LayoutVariant;
  isStudio: boolean;
}) {
  const isDark = useThemeStore((s) => s.resolvedTheme === 'dark');
  const isRefined = variant === 'refined';

  if (isRefined && isStudio) {
    return (
      <div
        className={cn(
          'w-52 border-r overflow-hidden flex-shrink-0 bg-background',
          isDark ? 'border-border' : 'border-border/40',
        )}
      >
        <FileSidebar />
      </div>
    );
  }

  if (isRefined) {
    return (
      <div className="w-56 border-r border-border/40 overflow-hidden flex-shrink-0">
        <FileSidebar />
      </div>
    );
  }

  return (
    <div className="w-56 lg:w-64 border-r border-border overflow-hidden flex-shrink-0">
      <FileSidebar />
    </div>
  );
}

/** 统一应用外壳：classic/refined × guided/studio */
export function AppShell() {
  const mode = useSettingsStore((s) => normalizeLayoutMode(s.layout.mode));
  const variant = useSettingsStore((s) => s.layout.variant ?? DEFAULT_LAYOUT_VARIANT);
  const shellTheme = useResolvedShellTheme();
  const isDark = useThemeStore((s) => s.resolvedTheme === 'dark');
  const setSidebarOpen = useUIStore((s) => s.setSidebarOpen);

  const isStudio = mode === 'studio';
  const isRefined = variant === 'refined';
  const isRefinedStudio = isRefined && isStudio;

  const defaultFileSidebar = isRefined ? isStudio : true;
  const [showFileSidebar, setShowFileSidebar] = useState(defaultFileSidebar);

  useShrinkBelow(1024, () => setSidebarOpen(false));
  useShrinkBelow(1280, () => {
    if (!isStudio && variant === 'classic') {
      setShowFileSidebar(false);
    }
  });

  useStudioRefinedBoot(isRefinedStudio);

  const themeProps = isRefined
    ? { theme: shellTheme, variant: 'refined' as const }
    : { theme: 'light' as const, variant: 'classic' as const };

  return (
    <ShellThemeProvider {...themeProps}>
      <div
        className={cn(
          'flex h-screen min-w-0',
          isRefined ? 'bg-background text-foreground' : '',
        )}
      >
        <Sidebar
          variant={variant}
          {...(isRefinedStudio ? { rail: true, theme: isDark ? 'studio' : 'default' } : {})}
        />
        <div
          className={cn(
            'flex-1 flex flex-col overflow-hidden min-w-0',
            isRefined && 'bg-background',
          )}
        >
          <ShellHeader
            variant={variant}
            isStudio={isStudio}
            showFileToggle
            showFileSidebar={showFileSidebar}
            onToggleFileSidebar={() => setShowFileSidebar((v) => !v)}
          />
          <ClaudeCliMissingBanner />
          <DashboardMainArea
            showFileSidebar={showFileSidebar}
            showContextPanel={isRefinedStudio}
            integratedTerminal={isRefinedStudio}
            outletClassName={isRefinedStudio ? 'min-h-full' : undefined}
            fileSidebar={<FileSidebarPanel variant={variant} isStudio={isStudio} />}
          />
          {isRefinedStudio ? <StudioRefinedStatusBar /> : <FactoryStatusBar />}
        </div>
        <DashboardOverlays />
      </div>
    </ShellThemeProvider>
  );
}
