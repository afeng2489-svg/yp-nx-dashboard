import { useState } from 'react';
import { ChevronDown, ChevronUp, Loader2, Paperclip, Send, X } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { FACTORY_INTENT_CHIPS } from '@/data/factoryIntentChips';
import { isP5LaunchPreviewEnabled, isP5FactoryAtEnabled } from '@/data/factoryFeatureFlags';
import { LaunchPreviewBar } from '@/components/factory/LaunchPreviewBar';
import { FactoryRoleAskPanel } from '@/components/factory/FactoryRoleAskPanel';
import { isVisibleInMoreDrawer } from '@/data/workflowTiers';
import {
  pipelineDurationHint,
  pipelineForWorkflow,
  pipelineLabelForWorkflow,
} from '@/data/workflowPipelines';
import { FACTORY_QUICK_LINES } from '@/data/factoryQuickStart';
import type { Workflow } from '@/stores/workflowStore';
import type { Team } from '@/stores/teamStore';
import type { FactoryAttachment } from '@/services/factoryAttachment';
import type { StackProfile } from '@/data/stackProfile';

type QuickCardItem = (typeof FACTORY_QUICK_LINES)[number] & { workflow: Workflow | null };

export interface FactoryIntentConsoleProps {
  intent: string;
  onIntentChange: (value: string) => void;
  loading: boolean;
  error: string | null;
  effectiveWorkflowName: string;
  selectedWorkflowName: string;
  onSelectedWorkflowChange: (name: string) => void;
  workflows: Workflow[];
  teams: Team[];
  overrideTeamId: string;
  onOverrideTeamChange: (teamId: string) => void;
  attachments: FactoryAttachment[];
  uploadingAttachment: boolean;
  onAttachmentPick: (file: File | null) => void;
  onRemoveAttachment: (relativePath: string) => void;
  currentProjectName?: string;
  onRun: (
    prompt: string,
    workflowName?: string,
    retryOpts?: {
      retryExecutionId?: string;
      retryFromStage?: string;
      skipQualityGateForStage?: string;
    },
  ) => void | Promise<{ ok: boolean; error?: string }>;
  onQuickLineLegacy: (item: QuickCardItem) => void;
  quickCards: QuickCardItem[];
  routingHint?: string;
  stackProfile?: StackProfile;
  onOpenNewProject?: () => void;
  teamId?: string;
  teamOpinions?: Array<{ roleName: string; content: string }>;
  onTeamReply?: (roleName: string, content: string) => void;
}

export function FactoryIntentConsole({
  intent,
  onIntentChange,
  loading,
  error,
  effectiveWorkflowName,
  selectedWorkflowName,
  onSelectedWorkflowChange,
  workflows,
  teams,
  overrideTeamId,
  onOverrideTeamChange,
  attachments,
  uploadingAttachment,
  onAttachmentPick,
  onRemoveAttachment,
  currentProjectName,
  onRun,
  onQuickLineLegacy,
  quickCards,
  routingHint,
  stackProfile,
  onOpenNewProject,
  teamId,
  teamOpinions = [],
  onTeamReply,
}: FactoryIntentConsoleProps) {
  const launchPreviewEnabled = isP5LaunchPreviewEnabled();
  const factoryAtEnabled = isP5FactoryAtEnabled();
  const [activeChipId, setActiveChipId] = useState<string | null>(null);
  const [showMore, setShowMore] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const pipeline = pipelineForWorkflow(effectiveWorkflowName);
  const pipelineLabel = pipelineLabelForWorkflow(effectiveWorkflowName);
  const durationHint = pipelineDurationHint(effectiveWorkflowName);

  const handleChip = (chip: (typeof FACTORY_INTENT_CHIPS)[number]) => {
    setActiveChipId(chip.id);
    if (chip.id === 'new-project') {
      onOpenNewProject?.();
      return;
    }
    if ('presetTask' in chip && chip.presetTask) {
      void onRun(chip.presetTask, chip.workflowName);
      return;
    }
    onSelectedWorkflowChange(chip.workflowName);
    if (!intent.trim() && 'placeholder' in chip && chip.placeholder) {
      onIntentChange('');
    }
  };

  return (
    <section
      className="rounded-2xl border border-border/60 bg-card shadow-sm p-6"
      data-testid="factory-intent-console"
    >
      <h2 className="text-lg font-semibold mb-1">今天想做什么？</h2>
      <p className="text-sm text-muted-foreground mb-4">
        输入想法，虚拟团队按产线推进；你当厂长，在审批节点拍板即可。
      </p>

      <div className="flex flex-wrap gap-2 mb-4">
        {FACTORY_INTENT_CHIPS.map((chip) => {
          const Icon = chip.icon;
          const active = activeChipId === chip.id;
          return (
            <button
              key={chip.id}
              type="button"
              data-testid="factory-intent-chip"
              onClick={() => handleChip(chip)}
              className={cn(
                'inline-flex items-center gap-2 px-3 py-2 rounded-xl border text-sm transition-colors',
                active
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-border/60 hover:border-primary/40 hover:bg-accent/50',
              )}
            >
              <Icon className="w-4 h-4 shrink-0" />
              <span className="font-medium">{chip.label}</span>
            </button>
          );
        })}
      </div>

      <div
        className="rounded-lg border border-border/50 bg-muted/25 px-3 py-2 mb-3 text-xs text-muted-foreground"
        data-testid="factory-pipeline-recommendation"
      >
        <span className="text-foreground font-medium">将使用 · {pipelineLabel}</span>
        <span className="mx-1.5">·</span>
        <span>{pipeline.length} 阶段</span>
        <span className="mx-1.5">·</span>
        <span>{durationHint}</span>
        {!selectedWorkflowName && intent.trim() && (
          <span className="ml-2 text-primary/80">（根据输入自动匹配）</span>
        )}
        {routingHint && (
          <span className="ml-2 text-sky-700 dark:text-sky-300">· {routingHint}</span>
        )}
      </div>

      {launchPreviewEnabled && intent.trim() && (
        <LaunchPreviewBar workflowName={effectiveWorkflowName} intent={intent} stack={stackProfile} />
      )}

      {factoryAtEnabled && teamId && (
        <FactoryRoleAskPanel teamId={teamId} onReply={onTeamReply} />
      )}

      {teamOpinions.length > 0 && (
        <div className="rounded-lg border border-border/40 bg-muted/20 px-3 py-2 text-xs space-y-1">
          <p className="font-medium text-foreground">团队意见</p>
          {teamOpinions.map((o, i) => (
            <p key={i} className="text-muted-foreground">
              <span className="text-primary">@{o.roleName}</span>：{o.content}
            </p>
          ))}
        </div>
      )}

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
                onClick={() => onRemoveAttachment(a.relativePath)}
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
          placeholder={
            activeChipId === 'new-project'
              ? '描述你想做的产品…'
              : '例如：给 README 增加快速开始章节 / 修复登录页 500 错误'
          }
          value={intent}
          onChange={(e) => onIntentChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              onRun(intent, effectiveWorkflowName);
            }
          }}
        />
        <Button
          type="button"
          disabled={!intent.trim() || loading}
          onClick={() => onRun(intent, effectiveWorkflowName)}
          className="self-end gap-2 px-5"
        >
          {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
          启动
        </Button>
      </div>

      {error && <p className="mt-2 text-sm text-destructive">{error}</p>}
      <p className="mt-2 text-xs text-muted-foreground">⌘/Ctrl + Enter 快速提交</p>

      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
        <button
          type="button"
          className="text-muted-foreground hover:text-foreground inline-flex items-center gap-1"
          onClick={() => setShowAdvanced((v) => !v)}
        >
          {showAdvanced ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
          高级选项
        </button>
        <button
          type="button"
          className="text-muted-foreground hover:text-foreground inline-flex items-center gap-1"
          onClick={() => setShowMore((v) => !v)}
        >
          {showMore ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
          更多方式
        </button>
        <Link to="/settings/ai" className="text-primary hover:underline">
          配置 Claude CLI
        </Link>
      </div>

      {showAdvanced && (
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs border-t border-border/40 pt-3">
          <select
            className="bg-background border border-border/50 rounded-lg px-2 py-1.5 max-w-[160px]"
            value={selectedWorkflowName}
            onChange={(e) => onSelectedWorkflowChange(e.target.value)}
            title="覆盖产线（高级）"
          >
            <option value="">自动匹配产线</option>
            {workflows.map((w) => (
              <option key={w.id} value={w.name}>
                {pipelineLabelForWorkflow(w.name)}
              </option>
            ))}
          </select>
          <select
            className="bg-background border border-border/50 rounded-lg px-2 py-1.5 max-w-[140px]"
            value={overrideTeamId}
            onChange={(e) => onOverrideTeamChange(e.target.value)}
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
                onAttachmentPick(e.target.files?.[0] ?? null);
                e.target.value = '';
              }}
            />
          </label>
          {currentProjectName && (
            <span className="text-muted-foreground truncate max-w-[120px]" title={currentProjectName}>
              工作区: {currentProjectName}
            </span>
          )}
        </div>
      )}

      {showMore && (
        <div className="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-2 border-t border-border/40 pt-3">
          {quickCards
            .filter(
              (item) =>
                !FACTORY_INTENT_CHIPS.some((c) => String(c.id) === String(item.id)) &&
                isVisibleInMoreDrawer(item.workflowName),
            )
            .map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  type="button"
                  data-testid="factory-quick-line"
                  onClick={() => onQuickLineLegacy(item)}
                  className="text-left p-3 rounded-lg border border-border/50 hover:border-primary/30 hover:bg-accent/30 transition-colors"
                >
                  <div className="flex items-center gap-2">
                    <Icon className="w-4 h-4 text-primary shrink-0" />
                    <span className="text-sm font-medium">{item.title}</span>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1 ml-6">{item.description}</p>
                </button>
              );
            })}
        </div>
      )}
    </section>
  );
}
