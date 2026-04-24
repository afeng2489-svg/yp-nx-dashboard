import { memo, useEffect, useRef } from 'react';
import { Terminal as TerminalIcon } from 'lucide-react';

interface EmbeddedTerminalPreviewProps {
  output: string;
  elapsedSecs: number;
  onViewTerminal: () => void;
}

export const EmbeddedTerminalPreview = memo(function EmbeddedTerminalPreview({
  output,
  elapsedSecs,
  onViewTerminal,
}: EmbeddedTerminalPreviewProps) {
  const scrollRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [output]);

  return (
    <div className="flex-1 max-w-[85%]">
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs text-muted-foreground flex items-center gap-1.5">
          <TerminalIcon className="w-3 h-3" />
          执行过程 ({elapsedSecs}s)
        </span>
        <button
          onClick={onViewTerminal}
          className="text-xs text-emerald-400 hover:text-emerald-300 transition-colors"
        >
          查看完整终端 →
        </button>
      </div>
      <div className="bg-[#1a1a1a] rounded-xl px-3 py-2 text-xs text-green-400 max-h-48 overflow-y-auto border border-white/5">
        <pre
          ref={scrollRef}
          className="whitespace-pre-wrap font-mono leading-relaxed"
        >
          {output || '等待输出...'}
        </pre>
      </div>
    </div>
  );
});
