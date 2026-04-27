import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { AlertTriangle, X } from 'lucide-react';
import { api } from '@/api/client';

const DISMISS_KEY = 'claude-cli-banner-dismissed';

/**
 * 全局横幅：未检测到 Claude Code CLI 时提示用户安装/配置。
 * 仅在 source=none 时显示；用户可以本会话内 dismiss。
 */
export function ClaudeCliMissingBanner() {
  const [missing, setMissing] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState<boolean>(() => {
    try {
      return sessionStorage.getItem(DISMISS_KEY) === '1';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    let cancelled = false;
    let attempt = 0;
    const maxAttempts = 30;

    const probe = async () => {
      while (!cancelled && attempt < maxAttempts) {
        attempt += 1;
        try {
          const cfg = await api.getClaudeCliConfig();
          if (!cancelled) {
            setMissing(cfg.source === 'none');
            setHint(cfg.install_hint);
          }
          return;
        } catch {
          await new Promise((r) => setTimeout(r, 1000));
        }
      }
    };

    void probe();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleDismiss = () => {
    try {
      sessionStorage.setItem(DISMISS_KEY, '1');
    } catch {
      // ignore
    }
    setDismissed(true);
  };

  if (!missing || dismissed) return null;

  return (
    <div className="px-4 py-2 bg-gradient-to-r from-amber-500/10 to-orange-500/10 border-b border-amber-500/30 flex items-center gap-3">
      <AlertTriangle className="w-4 h-4 text-amber-600 flex-shrink-0" />
      <div className="flex-1 text-sm text-amber-700 dark:text-amber-300">
        {hint ?? '未检测到 Claude Code CLI，工作流和团队对话将无法运行。'}
        <Link to="/ai-settings" className="ml-2 underline font-medium hover:text-amber-800">
          前往设置
        </Link>
      </div>
      <button
        onClick={handleDismiss}
        className="p-1 rounded hover:bg-amber-500/20 transition-colors"
        title="本会话不再提示"
      >
        <X className="w-4 h-4 text-amber-600" />
      </button>
    </div>
  );
}
