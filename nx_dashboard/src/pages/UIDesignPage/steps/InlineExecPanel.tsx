import { useEffect, useRef } from 'react';
import { Loader2, CheckCircle, ExternalLink, X, Copy } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useExecutionStore } from '@/stores/executionStore';
import type { RawLine } from '@/stores/executionStore';
import { showSuccess } from '@/lib/toast';
import { cn } from '@/lib/utils';

// ── 实时执行输出面板（读 store，无独立 WebSocket）────────
export function InlineExecPanel({
  executionId,
  onExtract,
  onClose,
}: {
  executionId: string;
  onExtract?: (key: string, value: string) => void;
  onClose?: () => void;
}) {
  const lines = useExecutionStore((s) => s.outputLines.get(executionId) ?? []);
  const execution = useExecutionStore((s) => s.executions.find((e) => e.id === executionId));
  const containerRef = useRef<HTMLDivElement>(null);
  const prevLenRef = useRef(0);
  const onExtractRef = useRef(onExtract);
  const navigate = useNavigate();

  useEffect(() => {
    onExtractRef.current = onExtract;
  }, [onExtract]);

  // 扫描新增行中的 EXTRACT: 模式
  useEffect(() => {
    const newLines = lines.slice(prevLenRef.current);
    prevLenRef.current = lines.length;
    for (const line of newLines) {
      if (line.type === 'output') {
        const m = line.content.match(/EXTRACT:(\w+)=(.+)/s);
        if (m) onExtractRef.current?.(m[1], m[2].trim());
      }
    }
  }, [lines]);

  // 自动滚到底部
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines]);

  const status = execution?.status ?? 'running';

  const copyAll = () => {
    const text = lines
      .map((l) => {
        if (l.type === 'stage_started') return `\n▶ ${l.stageName}`;
        if (l.type === 'stage_completed') return `✓ ${l.stageName}`;
        return l.content;
      })
      .join('\n');
    navigator.clipboard.writeText(text);
    showSuccess('已复制到剪贴板');
  };

  return (
    <div className="bg-card rounded-2xl border border-border/50 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border/50 bg-muted/30">
        <div className="flex items-center gap-2 text-sm font-medium">
          {(status === 'pending' || status === 'running') && (
            <Loader2 className="w-3.5 h-3.5 animate-spin text-blue-500" />
          )}
          {status === 'completed' && <CheckCircle className="w-3.5 h-3.5 text-green-500" />}
          {(status === 'failed' || status === 'cancelled') && (
            <X className="w-3.5 h-3.5 text-red-500" />
          )}
          <span>实时输出</span>
          <code className="text-xs text-muted-foreground font-mono">
            #{executionId.slice(0, 8)}
          </code>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={copyAll}
            title="复制全部"
            className="p-1.5 hover:bg-accent rounded-lg transition-colors"
          >
            <Copy className="w-3.5 h-3.5 text-muted-foreground" />
          </button>
          <button
            onClick={() => navigate('/executions')}
            title="查看完整执行记录"
            className="p-1.5 hover:bg-accent rounded-lg transition-colors"
          >
            <ExternalLink className="w-3.5 h-3.5 text-muted-foreground" />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 hover:bg-accent rounded-lg transition-colors"
            >
              <X className="w-3.5 h-3.5 text-muted-foreground" />
            </button>
          )}
        </div>
      </div>

      {/* 输出内容 */}
      <div
        ref={containerRef}
        className="max-h-96 overflow-y-auto p-4 space-y-0.5 font-mono text-xs bg-black/[0.03] dark:bg-white/[0.03]"
      >
        {lines.length === 0 && <p className="text-muted-foreground text-center py-6">等待输出…</p>}
        {lines.map((line: RawLine) => (
          <div
            key={line.id}
            className={cn(
              'leading-relaxed whitespace-pre-wrap break-words',
              line.type === 'stage_started' && 'text-blue-500 font-semibold pt-3 pb-0.5',
              line.type === 'stage_completed' && 'text-green-600 font-semibold pb-1',
              line.type === 'completed' && 'text-green-500 font-bold pt-2',
              line.type === 'error' && 'text-red-500',
              line.type === 'output' && 'text-foreground/75',
              line.type === 'info' && 'text-muted-foreground italic',
            )}
          >
            {line.type === 'stage_started' && `▶ ${line.stageName}`}
            {line.type === 'stage_completed' && `✓ ${line.stageName}`}
            {line.type !== 'stage_started' && line.type !== 'stage_completed' && line.content}
          </div>
        ))}
      </div>
    </div>
  );
}
