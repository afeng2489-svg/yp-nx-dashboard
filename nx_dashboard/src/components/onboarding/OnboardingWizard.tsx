import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { CheckCircle2, FolderOpen, Loader2, Rocket, Settings } from 'lucide-react';
import { api } from '@/api/client';
import { useClaudeCliReady } from '@/hooks/useClaudeCliReady';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { API_BASE_URL } from '@/api/constants';
import { unwrapEnvelope } from '@/api/response';
import { LAYOUT_MODES, type LayoutMode } from '@/data/layoutModes';
import { cn } from '@/lib/utils';
import type { Team } from '@/stores/teamStore';

const STORAGE_KEY = 'nexus-onboarding-v1';

export function isOnboardingComplete(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === 'done';
  } catch {
    return true;
  }
}

export function markOnboardingComplete() {
  try {
    localStorage.setItem(STORAGE_KEY, 'done');
  } catch {
    /* ignore */
  }
}

type Step = 'cli' | 'workspace' | 'team' | 'layout' | 'done';

/** AF-05 首次启动向导：CLI → 工作区 → Solo 团队 → /factory */
export function OnboardingWizard() {
  const navigate = useNavigate();
  const { ready: cliReady, config: cliConfig } = useClaudeCliReady();
  const { fetchWorkspaces, selectWorkspace, currentWorkspace } = useWorkspaceStore();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const { setCurrentTeam, fetchTeams } = useTeamStore();
  const setLayout = useSettingsStore((s) => s.setLayout);
  const layoutMode = useSettingsStore((s) => s.layout.mode ?? 'guided');
  const [open, setOpen] = useState(!isOnboardingComplete());
  const [step, setStep] = useState<Step>('cli');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) void fetchWorkspaces();
  }, [open, fetchWorkspaces]);

  const workspaceReady = Boolean(currentWorkspace?.root_path);

  useEffect(() => {
    if (!open || cliReady !== true) return;
    setStep((s) => (s === 'cli' ? 'workspace' : s));
  }, [cliReady, open]);

  useEffect(() => {
    if (!open || !workspaceReady) return;
    setStep((s) => (s === 'workspace' ? 'team' : s));
  }, [workspaceReady, open]);

  if (!open) return null;

  const finish = () => {
    markOnboardingComplete();
    setOpen(false);
    navigate('/factory');
  };

  const skip = () => {
    markOnboardingComplete();
    setOpen(false);
  };

  const createSoloTeam = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/teams/from-template`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ template: 'solo-fullstack' }),
      });
      if (!res.ok) throw new Error(`创建团队失败 (${res.status})`);
      const data = unwrapEnvelope<{ team: Team }>(await res.json());
      setCurrentTeam(data.team);
      await fetchTeams();
      setStep('layout');
    } catch (e) {
      setError(e instanceof Error ? e.message : '创建失败');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-background/80 backdrop-blur-sm p-4">
      <div className="w-full max-w-lg rounded-2xl border border-border bg-card shadow-xl p-6 space-y-5">
        <div>
          <h2 className="text-xl font-semibold">欢迎使用 AI 软件工厂</h2>
          <p className="text-sm text-muted-foreground mt-1">约 3 分钟完成设置，即可跑 Golden Path</p>
        </div>

        <ol className="space-y-3 text-sm">
          <StepRow
            done={cliReady === true}
            active={step === 'cli'}
            icon={Settings}
            title="1. Claude CLI"
            detail={
              cliReady === true
                ? `已绑定 ${cliConfig?.path ?? ''}`
                : '工厂 Run 需要本地 Claude Code CLI'
            }
            action={
              cliReady !== true ? (
                <Link to="/settings/ai" className="text-primary underline text-xs">
                  前往设置
                </Link>
              ) : null
            }
          />
          <StepRow
            done={!!currentWorkspace?.root_path}
            active={step === 'workspace'}
            icon={FolderOpen}
            title="2. 工作区"
            detail={
              currentWorkspace?.root_path
                ? currentWorkspace.name
                : '选择项目文件夹，CLI 在此读写代码'
            }
            action={
              !currentWorkspace?.root_path && workspaces.length > 0 ? (
                <button
                  type="button"
                  className="text-primary underline text-xs"
                  onClick={() => selectWorkspace(workspaces[0])}
                >
                  使用 {workspaces[0].name}
                </button>
              ) : null
            }
          />
          <StepRow
            done={step === 'layout' || step === 'done'}
            active={step === 'team'}
            icon={Rocket}
            title="3. Solo 团队"
            detail="一键创建全栈角色 + solo-dev 工作流"
          />
          <StepRow
            done={step === 'done'}
            active={step === 'layout'}
            icon={CheckCircle2}
            title="4. 布局模式"
            detail="选择最适合你的界面（可随时在设置中切换）"
          />
        </ol>

        {step === 'layout' && (
          <div className="grid grid-cols-1 gap-2">
            {LAYOUT_MODES.map((opt) => (
              <button
                key={opt.id}
                type="button"
                onClick={() => setLayout({ mode: opt.id as LayoutMode })}
                className={cn(
                  'text-left p-3 rounded-xl border transition-colors',
                  layoutMode === opt.id
                    ? 'border-primary bg-primary/5'
                    : 'border-border/50 hover:border-primary/40',
                )}
              >
                <p className="text-sm font-medium">{opt.label}</p>
                <p className="text-xs text-muted-foreground mt-0.5">{opt.description}</p>
              </button>
            ))}
          </div>
        )}

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div className="flex justify-between gap-2 pt-2">
          <button type="button" className="text-xs text-muted-foreground hover:underline" onClick={skip}>
            跳过
          </button>
          <div className="flex gap-2">
            {step === 'team' && (
              <button
                type="button"
                disabled={busy}
                className="btn-primary flex items-center gap-2"
                onClick={() => void createSoloTeam()}
              >
                {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : null}
                创建 Solo 团队
              </button>
            )}
            {step === 'layout' && (
              <button type="button" className="btn-primary flex items-center gap-2" onClick={() => setStep('done')}>
                继续
              </button>
            )}
            {step === 'done' && (
              <button type="button" className="btn-primary flex items-center gap-2" onClick={finish}>
                <CheckCircle2 className="w-4 h-4" />
                进入工厂台
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function StepRow({
  done,
  active,
  icon: Icon,
  title,
  detail,
  action,
}: {
  done: boolean;
  active: boolean;
  icon: typeof Settings;
  title: string;
  detail: string;
  action?: React.ReactNode;
}) {
  return (
    <li
      className={`flex gap-3 p-3 rounded-xl border ${
        active ? 'border-primary/40 bg-primary/5' : 'border-border/50'
      }`}
    >
      <Icon className={`w-5 h-5 shrink-0 ${done ? 'text-emerald-500' : 'text-muted-foreground'}`} />
      <div className="flex-1 min-w-0">
        <p className="font-medium">{title}</p>
        <p className="text-xs text-muted-foreground mt-0.5">{detail}</p>
        {action}
      </div>
      {done && <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" />}
    </li>
  );
}

/** 开发模式：检测 CLI 是否可用（供 e2e 脚本） */
export async function checkCliViaApi(): Promise<boolean> {
  try {
    const cfg = await api.getClaudeCliConfig();
    return cfg.source !== 'none' && !!cfg.path;
  } catch {
    return false;
  }
}
