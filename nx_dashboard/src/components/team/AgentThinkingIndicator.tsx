import { useEffect, useState } from 'react';
import { Bot, X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface AgentThinkingIndicatorProps {
  agentRole?: string;
  elapsedSecs: number;
  onCancel: () => void;
  partialOutput?: string;
}

/** Animated "agent is thinking" indicator with elapsed time and cancel button */
export function AgentThinkingIndicator({
  agentRole,
  elapsedSecs,
  onCancel,
  partialOutput,
}: AgentThinkingIndicatorProps) {
  const [dots, setDots] = useState('');

  // Animate dots
  useEffect(() => {
    const interval = setInterval(() => {
      setDots((prev) => (prev.length >= 3 ? '' : prev + '.'));
    }, 500);
    return () => clearInterval(interval);
  }, []);

  const label = agentRole && agentRole !== 'team' ? `${agentRole} 正在思考` : '正在思考';

  const showSlowWarning = elapsedSecs >= 30;

  return (
    <div className="flex gap-3 justify-start">
      {/* Pulsing avatar */}
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-emerald-500 to-green-500 flex items-center justify-center flex-shrink-0 animate-pulse">
        <Bot className="w-4 h-4 text-white" />
      </div>

      <div className="max-w-[80%] space-y-1.5">
        {/* Thinking bubble */}
        <div className="bg-muted rounded-2xl px-4 py-2.5">
          <div className="flex items-center gap-2">
            {/* Bouncing dots */}
            <span className="inline-flex gap-0.5">
              {[0, 1, 2].map((i) => (
                <span
                  key={i}
                  className={cn('w-1.5 h-1.5 rounded-full bg-emerald-500', 'animate-bounce')}
                  style={{ animationDelay: `${i * 150}ms` }}
                />
              ))}
            </span>
            <span className="text-sm text-muted-foreground">
              {label}
              {dots}
            </span>
          </div>

          {/* Elapsed time */}
          <div className="flex items-center gap-2 mt-1">
            <span className="text-xs text-muted-foreground/70">{elapsedSecs}s</span>
            {/* Cancel button */}
            <button
              onClick={onCancel}
              className="p-0.5 rounded hover:bg-red-500/20 text-red-400 transition-colors"
              title="取消"
            >
              <X className="w-3 h-3" />
            </button>
          </div>
        </div>

        {/* Slow warning */}
        {showSlowWarning && (
          <p className="text-xs text-amber-500/80 px-2">任务耗时较长，请耐心等待...</p>
        )}

        {/* Partial output preview */}
        {partialOutput && (
          <div className="bg-[#1a1a1a] rounded-xl px-3 py-2 text-xs text-green-400 max-h-48 overflow-y-auto border border-white/5">
            <pre className="whitespace-pre-wrap font-mono leading-relaxed">
              {partialOutput.slice(-2000)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
