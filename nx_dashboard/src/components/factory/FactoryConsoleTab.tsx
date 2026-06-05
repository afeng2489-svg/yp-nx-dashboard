import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Loader2, Paperclip, Send, X } from 'lucide-react';
import { useClaudeCliReady } from '@/hooks/useClaudeCliReady';
import {
  terminalFactoryRuns,
  useRecentFactoryOutcome,
} from '@/hooks/useRecentFactoryOutcome';
import { dismissRunOutcome } from '@/data/runNextSteps';
import { FACTORY_QUICK_LINES, suggestWorkflowName } from '@/data/factoryQuickStart';
import { autoRouteWorkflowWhenNoCli } from '@/data/textOnlyRouting';
import { requiresClaudeCliForWorkflow } from '@/data/textOnlyWorkflows';
import { CliSetupInline } from '@/components/factory/CliSetupInline';
import {
  getFirstRunChoice,
  isP5FailureRecoveryEnabled,
  isP5FirstRunWizardEnabled,
  isP5IntentConsoleEnabled,
  isP5RunNextStepEnabled,
  isP5TextOnlyFactoryEnabled,
} from '@/data/factoryFeatureFlags';
import {
  inferWorkspaceSignals,
  suggestWorkflowWithContext,
  filterFactoryQuickCards,
} from '@/data/workspaceSignals';
import { useAIConfigStore } from '@/stores/aiConfigStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import { ActiveExecutionsPanel } from '@/components/dashboard';
import { WorkflowLaunchModal } from '@/components/workflow/WorkflowLaunchModal';
import { SprintSnapshotStrip } from '@/components/factory/SprintSnapshotStrip';
import { RunPipelineBoard } from '@/components/factory/RunPipelineBoard';
import { RunCompleteBanner } from '@/components/factory/RunCompleteBanner';
import { FactoryIntentConsole } from '@/components/factory/FactoryIntentConsole';
import { FirstRunModal } from '@/components/factory/FirstRunModal';
import { NewProjectWizard } from '@/components/factory/NewProjectWizard';
import { FactoryTodoStrip } from '@/components/factory/FactoryTodoStrip';
import { ConsoleApprovalSummary } from '@/components/factory/ConsoleApprovalSummary';
import { FactoryMetricsStrip } from '@/components/factory/FactoryMetricsStrip';
import { cn } from '@/lib/utils';
import {
  isGoldenPathPrompt,
  recordFactoryEvent,
  saveRunMeta,
} from '@/services/factoryMetrics';
import { markSprintInProgress } from '@/services/sprintWriteback';
import { runFactoryQuickPrompt } from '@/services/factoryRun';
import {
  formatAttachmentsForPrompt,
  uploadFactoryAttachment,
  type FactoryAttachment,
} from '@/services/factoryAttachment';
import { Button } from '@/components/ui/button';
import type { Workflow } from '@/stores/workflowStore';
import { WorkspaceContextBar } from '@/components/factory/WorkspaceContextBar';
import { EditModeToggle, HumanEditHint } from '@/components/factory/EditModeToggle';

interface FactoryConsoleTabProps {
  onRunStarted?: () => void;
  initialIntent?: string;
  sprintId?: string;
  /** AF-11: full | guided-refined */
  variant?: 'full' | 'guided-refined';
}

export function FactoryConsoleTab({
  onRunStarted,
  initialIntent,
  sprintId,
  variant = 'full',
}: FactoryConsoleTabProps) {
  const isGuidedRefined = variant === 'guided-refined';
  const intentConsoleEnabled = isP5IntentConsoleEnabled(variant);
  const runNextStepEnabled = isP5RunNextStepEnabled();
  const failureRecoveryEnabled = isP5FailureRecoveryEnabled();
  const firstRunWizardEnabled = isP5FirstRunWizardEnabled();
  const navigate = useNavigate();
  const { workflows, fetchWorkflows } = useWorkflowStore();
  const { fetchExecutions, connectWebSocket } = useExecutionStore();
  const selectContextExecution = useContextPanelStore((s) => s.selectExecution);
  const currentWorkspace = useWorkspaceStore((s) => s.currentWorkspace);
  const workspaceFiles = useWorkspaceStore((s) => s.files);
  const gitStatus = useWorkspaceStore((s) => s.gitStatus);
  const browseFiles = useWorkspaceStore((s) => s.browseFiles);
  const fetchProvidersV2 = useAIConfigStore((s) => s.fetchProvidersV2);
  const openFactoryDrawer = useFactoryDrawerStore((s) => s.open);
  const factoryEditMode = useSettingsStore((s) => s.factoryEditMode);
  const recentOutcome = useRecentFactoryOutcome();
  const { teams, currentTeam, fetchTeams, setCurrentTeam } = useTeamStore();

  const [intent, setIntent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [launchWorkflow, setLaunchWorkflow] = useState<Workflow | null>(null);
  const [selectedWorkflowName, setSelectedWorkflowName] = useState('');
  const [overrideTeamId, setOverrideTeamId] = useState('');
  const [attachments, setAttachments] = useState<FactoryAttachment[]>([]);
  const [uploadingAttachment, setUploadingAttachment] = useState(false);
  const [showFirstRun, setShowFirstRun] = useState(false);
  const [showNewProjectWizard, setShowNewProjectWizard] = useState(false);
  const [teamOpinions, setTeamOpinions] = useState<Array<{ roleName: string; content: string }>>([]);
  const { ready: cliReady } = useClaudeCliReady();

  const workspaceSignals = useMemo(
    () =>
      inferWorkspaceSignals(
        currentWorkspace?.root_path,
        workspaceFiles,
        gitStatus?.branch,
      ),
    [currentWorkspace?.root_path, workspaceFiles, gitStatus?.branch],
  );

  const routing = useMemo(() => {
    if (selectedWorkflowName) {
      return { workflowName: selectedWorkflowName, hint: undefined as string | undefined };
    }
    if (!intent.trim()) {
      return { workflowName: 'solo-dev', hint: undefined as string | undefined };
    }
    const r = suggestWorkflowWithContext(intent, suggestWorkflowName, workspaceSignals);
    return { workflowName: r.workflowName, hint: r.hint };
  }, [intent, selectedWorkflowName, workspaceSignals]);

  useEffect(() => {
    fetchWorkflows();
    fetchTeams();
    void fetchProvidersV2();
    if (currentWorkspace?.root_path) void browseFiles();
  }, [fetchWorkflows, fetchTeams, fetchProvidersV2, browseFiles, currentWorkspace?.root_path]);

  useEffect(() => {
    if (firstRunWizardEnabled && intentConsoleEnabled && getFirstRunChoice() === null) {
      setShowFirstRun(true);
    }
  }, [firstRunWizardEnabled, intentConsoleEnabled]);

  useEffect(() => {
    if (initialIntent?.trim()) {
      setIntent(initialIntent.trim());
    }
  }, [initialIntent]);

  const quickCards = filterFactoryQuickCards(
    FACTORY_QUICK_LINES.map((item) => ({
      ...item,
      workflow: workflows.find((w) => w.name === item.workflowName) ?? null,
    })),
    workspaceSignals,
  );

  const hasActiveRun = useExecutionStore((s) =>
    s.executions.some((e) => e.status === 'running' || e.status === 'paused'),
  );

  const buildPrompt = (base: string) => {
    const parts = [base.trim()];
    const attachmentBlock = formatAttachmentsForPrompt(attachments);
    if (attachmentBlock) {
      parts.push(attachmentBlock);
    }
    return parts.join('');
  };

  const runQuickPrompt = async (
    prompt: string,
    workflowName?: string,
    opts?: {
      retryExecutionId?: string;
      retryFromStage?: string;
      skipQualityGateForStage?: string;
    },
  ): Promise<{ ok: boolean; error?: string }> => {
    const isRetry = Boolean(opts?.retryExecutionId);
    const fullPrompt = buildPrompt(prompt);

    if (loading) {
      const msg = '正在启动中，请稍候…';
      setError(msg);
      return { ok: false, error: msg };
    }
    if (!isRetry && !fullPrompt.trim()) {
      const msg = '请先输入任务描述';
      setError(msg);
      return { ok: false, error: msg };
    }

    let prelimWorkflow =
      workflowName ??
      (selectedWorkflowName ||
        (intent.trim()
          ? routing.workflowName
          : suggestWorkflowWithContext(prompt, suggestWorkflowName, workspaceSignals).workflowName));

    if (!opts?.retryExecutionId && cliReady === false && isP5TextOnlyFactoryEnabled()) {
      const routed = autoRouteWorkflowWhenNoCli(prelimWorkflow, fullPrompt, cliReady);
      prelimWorkflow = routed.workflowName;
      if (routed.hint) setError(routed.hint);
    }

    const needsCli = requiresClaudeCliForWorkflow(prelimWorkflow);
    if (cliReady === false && needsCli && !opts?.retryExecutionId) {
      const msg = isP5TextOnlyFactoryEnabled()
        ? '未绑定 Claude CLI。已尝试文本产线；改代码仍需配置 CLI'
        : '未绑定本地 Claude Code CLI，请先在设置中检测/配置路径';
      setError(msg);
      return { ok: false, error: msg };
    }
    if (!currentWorkspace?.root_path) {
      const msg = '请先在顶栏选择工作区（项目文件夹），Claude CLI 才能在该目录读写代码';
      setError(msg);
      return { ok: false, error: msg };
    }
    if (factoryEditMode === 'human' && hasActiveRun && !opts?.retryExecutionId) {
      const msg = '当前为「我来改」模式：请先在文件树手工编辑，或切回 Agent 改后再启动';
      setError(msg);
      return { ok: false, error: msg };
    }
    setLoading(true);
    setError(null);
    try {
      const resolvedWorkflow = prelimWorkflow;
      const teamId = overrideTeamId || currentTeam?.id;
      const result = await runFactoryQuickPrompt({
        prompt: isRetry ? prompt.trim() || '继续完成上次失败的任务' : fullPrompt,
        teamId,
        projectId: currentWorkspace?.id,
        workflowName: resolvedWorkflow,
        retryExecutionId: opts?.retryExecutionId,
        retryFromStage: opts?.retryFromStage,
        skipQualityGateForStage: opts?.skipQualityGateForStage,
      });
      if (result.ok) {
        if (!isRetry) {
          const prevOutcome = terminalFactoryRuns(useExecutionStore.getState().executions)[0];
          if (prevOutcome) dismissRunOutcome(prevOutcome.id);
        }
        const execId = result.executionId;
        const goldenPath = isGoldenPathPrompt(prompt);
        if (execId) {
          saveRunMeta(execId, {
            golden_path: goldenPath,
            started_at: Date.now(),
            sprint_id: sprintId,
          });
          if (sprintId) {
            void markSprintInProgress(sprintId, prompt.trim().slice(0, 120));
          }
          void recordFactoryEvent('run_started', {
            executionId: execId,
            payload: { golden_path: goldenPath, workflow: resolvedWorkflow },
          });
        }
        setIntent('');
        setAttachments([]);
        await fetchExecutions();
        if (execId) {
          connectWebSocket(execId);
          selectContextExecution(execId);
        }
        onRunStarted?.();
        return { ok: true };
      }
      const err = result.error ?? '启动失败';
      setError(err);
      return { ok: false, error: err };
    } catch (e) {
      const err = e instanceof Error ? e.message : '网络错误';
      setError(err);
      return { ok: false, error: err };
    } finally {
      setLoading(false);
    }
  };

  const handleAttachment = async (file: File | null) => {
    if (!file) return;
    if (!currentWorkspace?.root_path) {
      setError('请先选择工作区后再上传附件');
      return;
    }
    setUploadingAttachment(true);
    setError(null);
    try {
      const result = await uploadFactoryAttachment(file, currentWorkspace.id);
      if (result.ok) {
        setAttachments((prev) => [...prev, result.attachment]);
      } else {
        setError(result.error);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : '附件上传失败');
    } finally {
      setUploadingAttachment(false);
    }
  };

  const removeAttachment = (relativePath: string) => {
    setAttachments((prev) => prev.filter((a) => a.relativePath !== relativePath));
  };

  const handleQuickLine = (item: (typeof quickCards)[number]) => {
    const preset = 'presetTask' in item ? item.presetTask : undefined;
    if (preset) {
      void runQuickPrompt(preset, item.workflowName);
      return;
    }
    if (item.workflow) {
      setLaunchWorkflow(item.workflow);
      return;
    }
    setError(`工作流「${item.workflowName}」未找到，请先在资产库确认已加载`);
  };

  const effectiveWorkflow = routing.workflowName;

  const handleWizardStarted = async (executionId: string) => {
    await fetchExecutions();
    connectWebSocket(executionId);
    selectContextExecution(executionId);
    onRunStarted?.();
  };

  return (
    <div className="space-y-8">
      <WorkspaceContextBar />
      <div className="flex flex-wrap items-center gap-3">
        <EditModeToggle active={hasActiveRun} />
      </div>
      <HumanEditHint />

      {(failureRecoveryEnabled || intentConsoleEnabled) && <FactoryTodoStrip />}

      {!failureRecoveryEnabled && cliReady === false && (
        <div className="rounded-xl border border-amber-500/40 bg-amber-500/5 px-4 py-3 text-sm text-amber-800 dark:text-amber-200">
          工厂台 Run 依赖本地 <strong>Claude Code CLI</strong>（非云端 API）。
          <Link to="/settings/ai" className="ml-2 underline font-medium">前往绑定 CLI</Link>
        </div>
      )}
      {failureRecoveryEnabled && cliReady === false && <CliSetupInline />}
      {!currentWorkspace?.root_path && (
        <div className="rounded-xl border border-sky-500/30 bg-sky-500/5 px-4 py-3 text-sm text-sky-900 dark:text-sky-200">
          请先在顶栏左侧选择<strong>工作区文件夹</strong>，workflow 才会在正确目录调用 Claude CLI 改代码。
        </div>
      )}

      {intentConsoleEnabled ? (
        <>
          <FactoryIntentConsole
            intent={intent}
            onIntentChange={setIntent}
            loading={loading}
            error={error}
            effectiveWorkflowName={effectiveWorkflow}
            selectedWorkflowName={selectedWorkflowName}
            onSelectedWorkflowChange={setSelectedWorkflowName}
            workflows={workflows}
            teams={teams}
            overrideTeamId={overrideTeamId}
            onOverrideTeamChange={(teamId) => {
              setOverrideTeamId(teamId);
              const t = teams.find((x) => x.id === teamId);
              if (t) setCurrentTeam(t);
            }}
            attachments={attachments}
            uploadingAttachment={uploadingAttachment}
            onAttachmentPick={(file) => void handleAttachment(file)}
            onRemoveAttachment={removeAttachment}
            currentProjectName={currentWorkspace?.name}
            onRun={(prompt, wf, retryOpts) => runQuickPrompt(prompt, wf, retryOpts)}
            onQuickLineLegacy={handleQuickLine}
            quickCards={quickCards}
            routingHint={routing.hint}
            stackProfile={workspaceSignals.stack}
            onOpenNewProject={() => setShowNewProjectWizard(true)}
            teamId={overrideTeamId || currentTeam?.id}
            teamOpinions={teamOpinions}
            onTeamReply={(roleName, content) => {
              setTeamOpinions((prev) => [...prev, { roleName, content }]);
            }}
          />
          <SprintSnapshotStrip />
          <FactoryMetricsStrip />
        </>
      ) : (
        <section
          className={cn(
            'rounded-2xl border border-border/60 p-6',
            isGuidedRefined ? 'bg-card shadow-sm' : 'bg-gradient-to-br from-indigo-500/5 via-purple-500/5 to-pink-500/5',
          )}
        >
          <h2 className="text-lg font-semibold mb-1">一句话启动</h2>
          <p className="text-sm text-muted-foreground mb-4">
            你当厂长，虚拟团队按产线推进：开发 → 审批 → 审查 → 摘要
            {effectiveWorkflow && (
              <span className="ml-2 text-primary">→ 推荐: {effectiveWorkflow}</span>
            )}
          </p>

          <div className="flex flex-wrap items-center gap-2 mb-3 text-xs">
            <select
              className="bg-background border border-border/50 rounded-lg px-2 py-1.5 max-w-[160px]"
              value={selectedWorkflowName}
              onChange={(e) => setSelectedWorkflowName(e.target.value)}
              title="产线模板"
            >
              <option value="">自动匹配模板</option>
              {workflows.map((w) => (
                <option key={w.id} value={w.name}>
                  {w.name}
                </option>
              ))}
            </select>
            <select
              className="bg-background border border-border/50 rounded-lg px-2 py-1.5 max-w-[140px]"
              value={overrideTeamId}
              onChange={(e) => {
                setOverrideTeamId(e.target.value);
                const t = teams.find((x) => x.id === e.target.value);
                if (t) setCurrentTeam(t);
              }}
              title="覆盖团队"
            >
              <option value="">团队（顶栏）</option>
              {teams.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
            <label
              className={cn(
                'inline-flex items-center gap-1.5 px-2 py-1.5 rounded-lg border border-border/50 cursor-pointer hover:bg-accent/50',
                uploadingAttachment && 'opacity-60 pointer-events-none',
              )}
            >
              {uploadingAttachment ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Paperclip className="w-3.5 h-3.5" />
              )}
              附件
              <input
                type="file"
                className="hidden"
                disabled={uploadingAttachment}
                onChange={(e) => {
                  void handleAttachment(e.target.files?.[0] ?? null);
                  e.target.value = '';
                }}
              />
            </label>
            {currentWorkspace && (
              <span className="text-muted-foreground truncate max-w-[120px]" title={currentWorkspace.name}>
                工作区: {currentWorkspace.name}
              </span>
            )}
          </div>

          {attachments.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-2">
              {attachments.map((a) => (
                <span
                  key={a.relativePath}
                  className="inline-flex items-center gap-1 max-w-[200px] text-[11px] font-mono bg-muted/60 border border-border/40 rounded-md px-2 py-0.5"
                  title={a.path}
                >
                  <span className="truncate">{a.filename}</span>
                  <button
                    type="button"
                    onClick={() => removeAttachment(a.relativePath)}
                    className="shrink-0 p-0.5 rounded hover:bg-accent"
                    title="移除附件"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </span>
              ))}
            </div>
          )}

          <div className="flex gap-2">
            <textarea
              className="flex-1 rounded-xl px-4 py-3 resize-none focus:outline-none focus:ring-2 focus:ring-primary/20 border border-border/50 bg-background text-sm min-h-[88px]"
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
            <Button
              type="button"
              disabled={!intent.trim() || loading}
              onClick={() => void runQuickPrompt(intent)}
              className="self-end gap-2 px-5"
            >
              {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
              启动
            </Button>
          </div>
          {error && <p className="mt-2 text-sm text-destructive">{error}</p>}
          <p className="mt-2 text-xs text-muted-foreground">⌘/Ctrl + Enter 快速提交</p>
          <SprintSnapshotStrip />
          <FactoryMetricsStrip />
        </section>
      )}

      <RunPipelineBoard previewWorkflowName={effectiveWorkflow} />

      {runNextStepEnabled && recentOutcome && (
        <RunCompleteBanner
          execution={recentOutcome.execution}
          workflowName={recentOutcome.workflowName}
          stackProfile={workspaceSignals.stack}
          onPrefill={(prompt, wf) => {
            setIntent(prompt);
            if (wf) setSelectedWorkflowName(wf);
          }}
          onRun={(prompt, wf, retryOpts) => runQuickPrompt(prompt, wf, retryOpts)}
          onOpenTerminal={() => openFactoryDrawer('terminal')}
        />
      )}

      {!intentConsoleEnabled && (
        <section>
          <h3 className="text-sm font-medium text-muted-foreground mb-3">快捷产线</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
            {quickCards.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  type="button"
                  data-testid="factory-quick-line"
                  onClick={() => handleQuickLine(item)}
                  className={cn(
                    'group text-left p-4 rounded-xl border border-border/50 hover:border-primary/30',
                    'hover:shadow-md transition-all bg-card/50',
                  )}
                >
                  <div
                    className={cn(
                      'w-10 h-10 rounded-lg flex items-center justify-center mb-3',
                      isGuidedRefined
                        ? 'bg-primary/10 text-primary'
                        : cn('bg-gradient-to-br', item.gradient),
                    )}
                  >
                    <Icon className={cn('w-5 h-5', isGuidedRefined ? 'text-primary' : 'text-white')} />
                  </div>
                  <p className="font-medium text-sm">{item.title}</p>
                  <p className="text-xs text-muted-foreground mt-1">{item.description}</p>
                </button>
              );
            })}
          </div>
        </section>
      )}

      <ConsoleApprovalSummary />

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
        <ActiveExecutionsPanel variant="classic" />
      </section>

      {launchWorkflow && (
        <WorkflowLaunchModal
          workflow={launchWorkflow}
          onClose={() => setLaunchWorkflow(null)}
        />
      )}

      <FirstRunModal
        open={showFirstRun}
        onClose={() => setShowFirstRun(false)}
        onGreenfield={() => {
          setShowFirstRun(false);
          setShowNewProjectWizard(true);
        }}
        onExistingCode={() => setShowFirstRun(false)}
      />

      <NewProjectWizard
        open={showNewProjectWizard}
        onClose={() => setShowNewProjectWizard(false)}
        onStarted={(id) => void handleWizardStarted(id)}
      />
    </div>
  );
}
