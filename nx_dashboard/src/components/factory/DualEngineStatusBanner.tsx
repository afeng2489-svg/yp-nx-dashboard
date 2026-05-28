import { Link } from 'react-router-dom';
import { Terminal, Wifi, WifiOff } from 'lucide-react';
import { useClaudeCliReady } from '@/hooks/useClaudeCliReady';
import { useAIConfigStore } from '@/stores/aiConfigStore';
import { cn } from '@/lib/utils';

/** AF-MM-01：工厂内可见双引擎状态（代码车道 + 文本车道） */
export function DualEngineStatusBanner({ compact = false }: { compact?: boolean }) {
  const { ready: cliReady, config: cliConfig } = useClaudeCliReady();
  const providersV2 = useAIConfigStore((s) => s.providersV2);
  const enabledApi = providersV2.filter((p) => p.enabled && p.provider_key !== 'claude');

  const codeLane =
    cliReady === true
      ? `Claude CLI${cliConfig?.path ? '' : ''}`
      : cliReady === false
        ? '未配置'
        : '检测中…';

  const textLane =
    enabledApi.length > 0
      ? enabledApi.map((p) => p.name).slice(0, 2).join(' · ')
      : '未配置 API';

  if (compact) {
    return (
      <div
        className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground"
        data-testid="dual-engine-status"
      >
        <span className="inline-flex items-center gap-1">
          <Terminal className="h-3 w-3" />
          代码：<span className={cn(cliReady ? 'text-foreground' : 'text-amber-600')}>{codeLane}</span>
        </span>
        <span>·</span>
        <span className="inline-flex items-center gap-1">
          {enabledApi.length > 0 ? (
            <Wifi className="h-3 w-3 text-emerald-500" />
          ) : (
            <WifiOff className="h-3 w-3" />
          )}
          文本：<span className="text-foreground">{textLane}</span>
        </span>
        {cliReady === false && (
          <Link to="/settings/ai" className="text-primary hover:underline ml-1">
            配置
          </Link>
        )}
      </div>
    );
  }

  return (
    <div
      className={cn(
        'rounded-lg border px-3 py-2 text-xs',
        cliReady === false
          ? 'border-amber-500/30 bg-amber-500/5'
          : 'border-border/50 bg-muted/20',
      )}
      data-testid="dual-engine-status"
    >
      <p className="font-medium text-foreground mb-1">双引擎工厂</p>
      <p className="text-muted-foreground">
        <strong className="text-foreground">代码车道</strong>（改仓库 · 质量门）：
        <span className={cn('ml-1', cliReady ? 'text-emerald-700 dark:text-emerald-300' : 'text-amber-700')}>
          {codeLane}
        </span>
      </p>
      <p className="text-muted-foreground mt-0.5">
        <strong className="text-foreground">文本车道</strong>（摘要 · 计划 · 审查报告）：
        <span className="ml-1 text-foreground">{textLane}</span>
      </p>
      {cliReady === false && (
        <p className="mt-1.5 text-amber-800 dark:text-amber-200">
          可跑文本类任务；自动改代码需
          <Link to="/settings/ai" className="underline font-medium ml-1">
            配置 Claude CLI
          </Link>
        </p>
      )}
    </div>
  );
}
