import { ChevronRight, CheckCircle, XCircle, FileCode, Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { StageResult, StageOutput } from '@/stores/executionStore';
import { formatTime } from './utils';

export interface StageResultCardProps {
  result: StageResult;
  isExpanded: boolean;
  onToggle: () => void;
}

export function StageResultCard({ result, isExpanded, onToggle }: StageResultCardProps) {
  const qg = result.quality_gate_result;
  const hasQualityGate = qg !== undefined && qg !== null;

  return (
    <div className="border border-border/50 rounded-xl overflow-hidden bg-gradient-to-r from-card to-accent/20">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 hover:from-indigo-500/5 hover:to-purple-500/5 transition-all"
      >
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'p-1.5 rounded-lg transition-transform duration-200',
              isExpanded ? 'bg-emerald-500/10 rotate-90' : 'bg-indigo-500/10',
            )}
          >
            <ChevronRight
              className={cn(
                'w-4 h-4 text-indigo-500 transition-transform',
                isExpanded && 'rotate-90',
              )}
            />
          </div>
          {hasQualityGate ? (
            qg.passed ? (
              <CheckCircle className="w-4 h-4 text-emerald-500" />
            ) : (
              <XCircle className="w-4 h-4 text-red-500" />
            )
          ) : (
            <CheckCircle className="w-4 h-4 text-emerald-500" />
          )}
          <span className="font-medium">{result.stage_name}</span>
          {hasQualityGate && (
            <span
              className={cn(
                'text-xs px-2 py-0.5 rounded-full font-medium',
                qg.passed ? 'bg-emerald-500/10 text-emerald-400' : 'bg-red-500/10 text-red-400',
              )}
            >
              {qg.passed ? 'QA PASS' : 'QA FAIL'}
            </span>
          )}
          {hasQualityGate && qg.retry_count > 0 && (
            <span className="text-xs px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400 font-medium">
              重试 {qg.retry_count}
            </span>
          )}
        </div>
        <span className="text-sm text-muted-foreground">
          {result.completed_at ? formatTime(result.completed_at) : '进行中'}
        </span>
      </button>
      {isExpanded && (
        <div className="px-4 pb-4 space-y-2">
          {hasQualityGate && (
            <div className="space-y-1 mb-2">
              {qg.checks.map((check, idx) => (
                <div
                  key={idx}
                  className={cn(
                    'flex items-center gap-2 text-xs px-2 py-1.5 rounded-md border',
                    check.passed
                      ? 'bg-emerald-500/5 border-emerald-500/20 text-emerald-300'
                      : 'bg-red-500/5 border-red-500/20 text-red-300',
                  )}
                >
                  {check.passed ? (
                    <CheckCircle className="w-3 h-3 shrink-0" />
                  ) : (
                    <XCircle className="w-3 h-3 shrink-0" />
                  )}
                  <span className="font-mono">{check.cmd}</span>
                  <span className="ml-auto text-muted-foreground">{check.duration_ms}ms</span>
                </div>
              ))}
            </div>
          )}
          {hasQualityGate && qg.checks.some((c) => !c.passed && (c.stderr || c.stdout)) && (
            <div className="space-y-1 mb-2">
              {qg.checks
                .filter((c) => !c.passed)
                .map((check, idx) => (
                  <div key={`err-${idx}`} className="space-y-1">
                    {check.stderr && (
                      <details className="text-xs">
                        <summary className="text-red-400 cursor-pointer hover:text-red-300">
                          {check.cmd} — stderr
                        </summary>
                        <pre className="mt-1 p-2 bg-[#1e1e1e] rounded-md text-gray-400 font-mono whitespace-pre-wrap max-h-40 overflow-auto border border-white/5">
                          {check.stderr}
                        </pre>
                      </details>
                    )}
                    {check.stdout && (
                      <details className="text-xs">
                        <summary className="text-amber-400 cursor-pointer hover:text-amber-300">
                          {check.cmd} — stdout
                        </summary>
                        <pre className="mt-1 p-2 bg-[#1e1e1e] rounded-md text-gray-400 font-mono whitespace-pre-wrap max-h-40 overflow-auto border border-white/5">
                          {check.stdout}
                        </pre>
                      </details>
                    )}
                  </div>
                ))}
            </div>
          )}
          {result.outputs &&
            result.outputs.length > 0 &&
            result.outputs.map((output, idx) => <StageOutputItem key={idx} output={output} />)}
        </div>
      )}
    </div>
  );
}

function StageOutputItem({ output }: { output: StageOutput }) {
  const hasStructured = output.summary || (output.files_changed && output.files_changed.length > 0);
  const rawText =
    typeof output === 'string'
      ? output
      : output.content
        ? output.content
        : JSON.stringify(output, null, 2);

  return (
    <div className="space-y-2">
      {/* 结构化摘要卡片 */}
      {hasStructured && (
        <div className="p-3 rounded-lg bg-gradient-to-r from-indigo-500/5 to-purple-500/5 border border-indigo-500/20 space-y-2">
          {output.summary && (
            <div className="flex items-start gap-2">
              <Sparkles className="w-3.5 h-3.5 text-indigo-400 mt-0.5 shrink-0" />
              <p className="text-sm text-gray-200 leading-relaxed">{output.summary}</p>
            </div>
          )}
          {output.files_changed && output.files_changed.length > 0 && (
            <div className="flex items-start gap-2">
              <FileCode className="w-3.5 h-3.5 text-purple-400 mt-0.5 shrink-0" />
              <div className="flex flex-wrap gap-1">
                {output.files_changed.map((file, i) => (
                  <span
                    key={i}
                    className="text-xs px-2 py-0.5 rounded-full bg-purple-500/10 text-purple-300 font-mono border border-purple-500/20"
                  >
                    {file}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* 原始文本输出（可折叠） */}
      <details className="text-xs">
        <summary className="text-muted-foreground cursor-pointer hover:text-gray-400 transition-colors">
          {hasStructured ? '查看原始输出' : '查看输出'}
        </summary>
        <pre className="mt-2 p-3 bg-[#1e1e1e] rounded-lg text-gray-400 font-mono whitespace-pre-wrap max-h-80 overflow-auto border border-white/5">
          {rawText}
        </pre>
      </details>
    </div>
  );
}
