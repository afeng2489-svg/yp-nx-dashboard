import { useState } from 'react';
import { CheckCircle2, AlertTriangle, X, ArrowRight, RotateCcw, Loader2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  dismissRunOutcome,
  nextStepsForRun,
  type RunNextStepAction,
} from '@/data/runNextSteps';
import type { Execution } from '@/stores/executionStore';
import { recordFactoryEvent } from '@/services/factoryMetrics';
import { showError, showSuccess } from '@/lib/toast';

export interface RunCompleteBannerProps {
  execution: Execution;
  workflowName: string;
  onPrefill: (prompt: string, workflowName?: string) => void;
  onRun: (
    prompt: string,
    workflowName?: string,
    opts?: {
      retryExecutionId?: string;
      retryFromStage?: string;
      skipQualityGateForStage?: string;
    },
  ) => Promise<{ ok: boolean; error?: string }>;
  onOpenTerminal?: () => void;
}

/** AF-UX-03 + AF-UX-09：Run 完成 / 失败态 Banner + 主 CTA */
export function RunCompleteBanner(props: RunCompleteBannerProps) {
  const { execution, workflowName } = props;
  const steps = nextStepsForRun(execution, workflowName);
  const [acting, setActing] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  if (!steps) return null;

  const isFailed = execution.status === 'failed';

  const handleAction = async (action: RunNextStepAction) => {
    void recordFactoryEvent('run_completed', {
      executionId: execution.id,
      payload: { next_step: action.kind, label: action.label },
    });

    setActionError(null);

    switch (action.kind) {
      case 'prefill':
        props.onPrefill(action.prompt ?? '', action.workflowName);
        showSuccess('已填入任务描述，确认后点启动');
        break;
      case 'run':
      case 'retry': {
        setActing(true);
        try {
          const result = await props.onRun(action.prompt ?? '', action.workflowName, {
            retryExecutionId: action.retryExecutionId,
            retryFromStage: action.retryFromStage,
            skipQualityGateForStage: action.skipQualityGateForStage,
          });
          if (result.ok) {
            showSuccess(action.kind === 'retry' ? '已重新启动 Run' : '已启动新 Run');
            dismissRunOutcome(execution.id);
          } else {
            const msg = result.error ?? (action.kind === 'retry' ? '重试失败' : '启动失败');
            setActionError(msg);
            showError(msg);
          }
        } catch (e) {
          const msg = e instanceof Error ? e.message : '操作失败';
          setActionError(msg);
          showError(msg);
        } finally {
          setActing(false);
        }
        break;
      }
      case 'navigate':
        if (action.href?.includes('drawer=terminal')) {
          props.onOpenTerminal?.();
        }
        dismissRunOutcome(execution.id);
        break;
      default:
        break;
    }
  };

  return (
    <section
      className={cn(
        'rounded-2xl border px-5 py-4 shadow-sm',
        isFailed
          ? 'border-destructive/40 bg-destructive/5'
          : 'border-emerald-500/40 bg-emerald-500/5',
      )}
      data-testid="run-complete-banner"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0">
          {isFailed ? (
            <AlertTriangle className="h-5 w-5 text-destructive shrink-0 mt-0.5" />
          ) : (
            <CheckCircle2 className="h-5 w-5 text-emerald-600 dark:text-emerald-400 shrink-0 mt-0.5" />
          )}
          <div className="min-w-0">
            <h3 className="text-sm font-semibold">{steps.title}</h3>
            {steps.summary && (
              <p className="text-sm text-muted-foreground mt-1 line-clamp-3">{steps.summary}</p>
            )}
            {steps.filesChanged != null && steps.filesChanged > 0 && !isFailed && (
              <p className="text-xs text-muted-foreground mt-1">
                变更 {steps.filesChanged} 个文件
              </p>
            )}
          </div>
        </div>
        <button
          type="button"
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent shrink-0"
          title="关闭"
          onClick={() => dismissRunOutcome(execution.id)}
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {actionError && (
        <p className="text-sm text-destructive mt-2" data-testid="run-banner-action-error">
          {actionError}
        </p>
      )}

      <div className="flex flex-wrap items-center gap-2 mt-4">
        <Button
          type="button"
          size="sm"
          variant={isFailed ? 'destructive' : 'default'}
          className="gap-1.5"
          disabled={acting}
          data-testid="run-next-step-primary"
          onClick={() => void handleAction(steps.primary)}
        >
          {acting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : isFailed ? (
            <RotateCcw className="h-3.5 w-3.5" />
          ) : (
            <ArrowRight className="h-3.5 w-3.5" />
          )}
          {steps.primary.label}
        </Button>

        {steps.secondary &&
          (steps.secondary.kind === 'navigate' && steps.secondary.href ? (
            <Button size="sm" variant="outline" asChild disabled={acting} data-testid="run-next-step-secondary">
              <Link
                to={steps.secondary.href}
                onClick={() => {
                  if (steps.secondary?.href?.includes('drawer=terminal')) {
                    props.onOpenTerminal?.();
                  }
                  dismissRunOutcome(execution.id);
                }}
              >
                {steps.secondary.label}
              </Link>
            </Button>
          ) : (
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={acting}
              data-testid="run-next-step-secondary"
              onClick={() => void handleAction(steps.secondary!)}
            >
              {steps.secondary.label}
            </Button>
          ))}

        {steps.tertiary && (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            className="text-muted-foreground"
            disabled={acting}
            data-testid="run-next-step-tertiary"
            onClick={() => void handleAction(steps.tertiary!)}
          >
            {steps.tertiary.label}
          </Button>
        )}
      </div>
    </section>
  );
}
