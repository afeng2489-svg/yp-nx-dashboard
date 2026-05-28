import { useState } from 'react';
import { Link } from 'react-router-dom';
import { CheckCircle2, Loader2, Terminal } from 'lucide-react';
import { api } from '@/api/client';
import { Button } from '@/components/ui/button';

/** AF-UX-09：工厂内嵌 Claude CLI 三步引导（检测 → 路径 → 再试） */
export function CliSetupInline({ onReady }: { onReady?: () => void }) {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [detecting, setDetecting] = useState(false);
  const [path, setPath] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  const detect = async () => {
    setDetecting(true);
    setMessage(null);
    try {
      const cfg = await api.detectClaudeCli();
      if (cfg.path) {
        setPath(cfg.path);
        setStep(2);
        setMessage('已检测到 Claude CLI');
      } else {
        setMessage(cfg.install_hint ?? '未检测到 CLI，请手动填写路径或安装 Claude Code');
        setStep(2);
      }
    } catch (e) {
      setMessage(e instanceof Error ? e.message : '检测失败');
    } finally {
      setDetecting(false);
    }
  };

  const save = async () => {
    if (!path.trim()) return;
    setDetecting(true);
    try {
      await api.setClaudeCliPath(path.trim());
      setStep(3);
      setReady(true);
      setMessage('CLI 已保存，可以启动改代码类任务');
      onReady?.();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : '保存失败');
    } finally {
      setDetecting(false);
    }
  };

  return (
    <div
      className="rounded-lg border border-amber-500/30 bg-amber-500/5 px-3 py-2.5 text-xs space-y-2"
      data-testid="cli-setup-inline"
    >
      <p className="font-medium text-amber-900 dark:text-amber-100 flex items-center gap-1.5">
        <Terminal className="h-3.5 w-3.5" />
        配置 Claude CLI（{step}/3）
      </p>
      {step === 1 && (
        <Button type="button" size="sm" variant="outline" className="h-7 text-xs" onClick={() => void detect()} disabled={detecting}>
          {detecting ? <Loader2 className="h-3 w-3 animate-spin mr-1" /> : null}
          1. 自动检测
        </Button>
      )}
      {step >= 2 && (
        <div className="flex flex-wrap gap-2 items-center">
          <input
            className="flex-1 min-w-[180px] rounded border border-border px-2 py-1 bg-background text-xs"
            placeholder="CLI 路径，如 /usr/local/bin/claude"
            value={path}
            onChange={(e) => setPath(e.target.value)}
          />
          <Button type="button" size="sm" className="h-7 text-xs" onClick={() => void save()} disabled={detecting || !path.trim()}>
            2. 保存路径
          </Button>
        </div>
      )}
      {ready && (
        <p className="text-emerald-700 dark:text-emerald-300 flex items-center gap-1">
          <CheckCircle2 className="h-3.5 w-3.5" />
          3. 就绪 — 可重新启动 Run
        </p>
      )}
      {message && <p className="text-muted-foreground">{message}</p>}
      <Link to="/settings/ai" className="text-primary hover:underline">
        打开 AI 设置
      </Link>
    </div>
  );
}
