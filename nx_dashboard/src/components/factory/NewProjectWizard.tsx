import { useState } from 'react';
import { Loader2 } from 'lucide-react';
import { mkdir } from '@tauri-apps/plugin-fs';
import { pickDirectory } from '@/lib/pickDirectory';
import { isTauri } from '@/lib/tauriEnv';
import { join } from '@tauri-apps/api/path';
import { Button } from '@/components/ui/button';
import {
  GREENFIELD_STACK_PRESETS,
  type GreenfieldStackId,
  stackHintForPreset,
} from '@/data/greenfieldStacks';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { runFactoryQuickPrompt } from '@/services/factoryRun';
import { cn } from '@/lib/utils';

export interface NewProjectWizardProps {
  open: boolean;
  onClose: () => void;
  onStarted?: (executionId: string) => void;
}

type Step = 1 | 2 | 3;

/** AF-UX-01 + AF-16：新建项目 3 步向导 */
export function NewProjectWizard({ open, onClose, onStarted }: NewProjectWizardProps) {
  const { createWorkspace, selectWorkspace } = useWorkspaceStore();
  const [step, setStep] = useState<Step>(1);
  const [projectName, setProjectName] = useState('');
  const [parentPath, setParentPath] = useState('');
  const [description, setDescription] = useState('');
  const [stack, setStack] = useState<GreenfieldStackId>('auto');
  const [loading, setLoading] = useState(false);
  const [pickingFolder, setPickingFolder] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  /** macOS：模态窗会挡住系统文件夹对话框，选目录时先卸掉向导 UI */
  if (pickingFolder) {
    return (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-background/40 p-4"
        data-testid="new-project-wizard-picking"
      >
        <p className="text-sm text-muted-foreground">正在打开文件夹选择器…</p>
      </div>
    );
  }

  const reset = () => {
    setStep(1);
    setProjectName('');
    setParentPath('');
    setDescription('');
    setStack('auto');
    setError(null);
  };

  const handlePickParent = async () => {
    setError(null);
    setPickingFolder(true);
    // 等模态 DOM 卸掉后再调原生对话框（与顶栏选择文件夹同路径）
    await new Promise<void>((r) => setTimeout(r, 120));
    try {
      const result = await pickDirectory('选择项目存放位置');
      if (result.ok) {
        setParentPath(result.path);
      } else if (!result.cancelled) {
        setError(
          `无法打开文件夹选择器：${result.error}。请直接在输入框填写路径（例如 /Users/qinyu/Code）`,
        );
      }
    } finally {
      setPickingFolder(false);
    }
  };

  const handleLaunch = async () => {
    const name = projectName.trim();
    const desc = description.trim();
    if (!name || !desc) {
      setError('请填写项目名和描述');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      let rootPath = parentPath.trim();
      if (rootPath) {
        const fullPath = await join(rootPath, name);
        await mkdir(fullPath, { recursive: true });
        rootPath = fullPath;
      }

      if (rootPath) {
        const ws = await createWorkspace(name, desc, rootPath);
        if (ws) selectWorkspace(ws);
      }

      const prompt = [
        `项目名：${name}`,
        `描述：${desc}`,
        stack !== 'auto' ? `技术栈：${stack}` : '',
        stackHintForPreset(stack),
      ]
        .filter(Boolean)
        .join('\n');

      const result = await runFactoryQuickPrompt({
        prompt,
        workflowName: 'greenfield-mvp',
      });

      if (result.ok && result.executionId) {
        onStarted?.(result.executionId);
        reset();
        onClose();
      } else {
        setError(result.error ?? '启动失败');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : '创建项目失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4"
      data-testid="new-project-wizard"
    >
      <div className="w-full max-w-md rounded-2xl border border-border bg-card shadow-xl p-6 space-y-4">
        <div>
          <h2 className="text-lg font-semibold">新建项目</h2>
          <p className="text-xs text-muted-foreground">步骤 {step} / 3</p>
        </div>

        {step === 1 && (
          <div className="space-y-3">
            <div>
              <label htmlFor="wizard-project-name" className="text-sm font-medium">
                项目名
              </label>
              <input
                id="wizard-project-name"
                className="mt-1 w-full rounded-lg border border-border px-3 py-2 text-sm bg-background"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                placeholder="my-app"
              />
            </div>
            <div>
              <label className="text-sm font-medium">存放位置</label>
              <div className="mt-1 flex gap-2">
                <input
                  className="flex-1 rounded-lg border border-border px-3 py-2 text-sm bg-background font-mono text-xs"
                  value={parentPath}
                  onChange={(e) => setParentPath(e.target.value)}
                  placeholder="/Users/you/projects"
                />
                <Button type="button" variant="outline" size="sm" onClick={() => void handlePickParent()}>
                  浏览
                </Button>
              </div>
              <p className="text-[11px] text-muted-foreground mt-1">
                将创建 {parentPath ? `${parentPath}/${projectName || '…'}` : '…'} 并绑定工作区
              </p>
              {!isTauri() && (
                <p className="text-[11px] text-amber-600 dark:text-amber-400 mt-1">
                  浏览器模式请手填路径；桌面版可点「浏览」
                </p>
              )}
              {isTauri() && (
                <p className="text-[11px] text-muted-foreground mt-0.5">
                  与顶栏「选择文件夹」相同；若无响应请手填路径
                </p>
              )}
            </div>
          </div>
        )}

        {step === 2 && (
          <div>
            <label htmlFor="wizard-description" className="text-sm font-medium">
              一句话描述
            </label>
            <textarea
              id="wizard-description"
              className="mt-1 w-full rounded-lg border border-border px-3 py-2 text-sm bg-background min-h-[88px]"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="一个本地优先的 Todo App"
            />
          </div>
        )}

        {step === 3 && (
          <div className="space-y-2">
            <p className="text-sm font-medium">技术栈</p>
            {GREENFIELD_STACK_PRESETS.map((preset) => (
              <button
                key={preset.id}
                type="button"
                className={cn(
                  'w-full text-left p-3 rounded-lg border text-sm transition-colors',
                  stack === preset.id
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/30',
                )}
                onClick={() => setStack(preset.id)}
              >
                <span className="font-medium">{preset.label}</span>
                <span className="text-xs text-muted-foreground ml-2">{preset.description}</span>
              </button>
            ))}
          </div>
        )}

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div className="flex justify-between gap-2 pt-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              if (step === 1) {
                reset();
                onClose();
              } else {
                setStep((s) => (s - 1) as Step);
              }
            }}
          >
            {step === 1 ? '取消' : '上一步'}
          </Button>
          {step < 3 ? (
            <Button
              type="button"
              size="sm"
              disabled={step === 1 && !projectName.trim()}
              onClick={() => setStep((s) => (s + 1) as Step)}
            >
              下一步
            </Button>
          ) : (
            <Button
              type="button"
              size="sm"
              disabled={loading || !description.trim()}
              onClick={() => void handleLaunch()}
              className="gap-2"
            >
              {loading && <Loader2 className="h-4 w-4 animate-spin" />}
              启动
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
