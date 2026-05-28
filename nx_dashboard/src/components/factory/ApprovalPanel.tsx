import { useEffect, useMemo, useState } from 'react';
import { CheckCircle2, Loader2, XCircle } from 'lucide-react';
import { useExecutionStore } from '@/stores/executionStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { api } from '@/api/client';
import { cn } from '@/lib/utils';

interface ApprovalPanelProps {
  executionId: string;
  stageName: string;
  question: string;
  compact?: boolean;
}

type PreviewMode = 'json' | 'tree' | 'diff';

function tryParseJson(text: string): unknown | null {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function JsonTree({ data, depth = 0 }: { data: unknown; depth?: number }) {
  if (data === null || data === undefined) {
    return <span className="text-muted-foreground">null</span>;
  }
  if (typeof data !== 'object') {
    return <span className="font-mono text-xs">{String(data)}</span>;
  }
  if (Array.isArray(data)) {
    return (
      <ul className="ml-3 space-y-0.5 border-l border-border/40 pl-2">
        {data.map((item, i) => (
          <li key={i} className="text-xs">
            <span className="text-muted-foreground">[{i}]</span>{' '}
            <JsonTree data={item} depth={depth + 1} />
          </li>
        ))}
      </ul>
    );
  }
  return (
    <div className="space-y-1">
      {Object.entries(data as Record<string, unknown>).map(([key, val]) => (
        <div key={key} className="text-xs">
          <span className="font-medium text-primary">{key}</span>
          <div className="ml-2">
            <JsonTree data={val} depth={depth + 1} />
          </div>
        </div>
      ))}
    </div>
  );
}

export function ApprovalPanel({
  executionId,
  stageName,
  question,
  compact = false,
}: ApprovalPanelProps) {
  const resolveExecution = useExecutionStore((s) => s.resolveExecution);
  const getExecution = useExecutionStore((s) => s.getExecution);
  const executions = useExecutionStore((s) => s.executions);
  const selectExecution = useContextPanelStore((s) => s.selectExecution);
  const [comment, setComment] = useState('');
  const [loading, setLoading] = useState<'approve' | 'reject' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [previewMode, setPreviewMode] = useState<PreviewMode>('json');
  const [artifactPreview, setArtifactPreview] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const execution = useMemo(
    () => executions.find((e) => e.id === executionId),
    [executions, executionId],
  );

  useEffect(() => {
    let cancelled = false;
    setPreviewLoading(true);
    (async () => {
      try {
        const ex = execution?.stage_results?.length
          ? execution
          : await getExecution(executionId);
        const stageOutput = ex?.stage_results
          ?.find((s) => s.stage_name === stageName)
          ?.outputs?.[0]?.content?.trim();
        if (stageOutput) {
          if (!cancelled) setArtifactPreview(stageOutput);
          return;
        }
        const files = await api.listArtifacts(executionId, stageName);
        const first = Array.isArray(files) ? files[0] : null;
        if (first?.relative_path) {
          const file = await api.getArtifactContent(executionId, first.relative_path);
          if (!cancelled) setArtifactPreview(file.content ?? null);
        } else if (!cancelled) {
          setArtifactPreview(null);
        }
      } catch {
        if (!cancelled) setArtifactPreview(null);
      } finally {
        if (!cancelled) setPreviewLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [executionId, stageName, execution, getExecution]);

  const parsedJson = useMemo(
    () => (artifactPreview ? tryParseJson(artifactPreview) : null),
    [artifactPreview],
  );

  const handleResolve = async (approved: boolean) => {
    setLoading(approved ? 'approve' : 'reject');
    setError(null);
    try {
      await resolveExecution(executionId, approved, comment.trim() || undefined);
      if (!compact) selectExecution(executionId);
    } catch (e) {
      setError(e instanceof Error ? e.message : '审批失败');
    } finally {
      setLoading(null);
    }
  };

  return (
    <div
      className={cn(
        'rounded-xl border border-amber-500/30 bg-amber-500/5',
        compact ? 'p-3' : 'p-4',
      )}
    >
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-lg bg-amber-500/10 shrink-0">
          <CheckCircle2 className="w-4 h-4 text-amber-600" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium text-sm">待审批</p>
          <p className="text-xs text-muted-foreground mt-0.5">阶段: {stageName}</p>
          <p className="text-sm mt-2 leading-relaxed">{question}</p>
          <p className="text-[11px] text-muted-foreground mt-1 font-mono">
            Run {executionId.slice(0, 8)}…
          </p>

          {(previewLoading || artifactPreview) && (
            <div className="mt-3 rounded-lg border border-border/50 bg-background/80 overflow-hidden">
              <div className="flex border-b border-border/40 text-[10px]">
                {(['json', 'tree', 'diff'] as PreviewMode[]).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    onClick={() => setPreviewMode(mode)}
                    className={cn(
                      'px-2.5 py-1.5 uppercase tracking-wide',
                      previewMode === mode
                        ? 'bg-primary/10 text-primary font-medium'
                        : 'text-muted-foreground hover:bg-accent/50',
                    )}
                  >
                    {mode === 'json' ? 'JSON' : mode === 'tree' ? '可视化' : 'Diff'}
                  </button>
                ))}
              </div>
              <div className="max-h-40 overflow-auto p-2 text-xs font-mono">
                {previewLoading ? (
                  <span className="text-muted-foreground inline-flex items-center gap-1">
                    <Loader2 className="w-3 h-3 animate-spin" /> 加载产物…
                  </span>
                ) : previewMode === 'tree' && parsedJson ? (
                  <JsonTree data={parsedJson} />
                ) : previewMode === 'diff' ? (
                  <pre className="whitespace-pre-wrap text-[11px] text-muted-foreground">
                    {artifactPreview?.slice(0, 2000) ?? '无 diff 内容'}
                  </pre>
                ) : (
                  <pre className="whitespace-pre-wrap text-[11px]">
                    {parsedJson
                      ? JSON.stringify(parsedJson, null, 2)
                      : artifactPreview?.slice(0, 4000) ?? '暂无 stage 产物'}
                  </pre>
                )}
              </div>
            </div>
          )}

          <textarea
            className="mt-3 w-full min-h-[56px] rounded-lg border border-border/60 bg-background px-3 py-2 text-xs resize-none"
            placeholder="审批意见（驳回时建议填写）"
            value={comment}
            onChange={(e) => setComment(e.target.value)}
          />

          {error && <p className="mt-2 text-xs text-destructive">{error}</p>}

          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              disabled={loading !== null}
              onClick={() => void handleResolve(true)}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-50"
            >
              {loading === 'approve' ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <CheckCircle2 className="w-3.5 h-3.5" />
              )}
              批准
            </button>
            <button
              type="button"
              disabled={loading !== null}
              onClick={() => void handleResolve(false)}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-red-500/40 text-red-600 hover:bg-red-500/5 disabled:opacity-50"
            >
              {loading === 'reject' ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <XCircle className="w-3.5 h-3.5" />
              )}
              驳回
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
