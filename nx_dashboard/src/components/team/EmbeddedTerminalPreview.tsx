import { memo, useEffect, useRef } from 'react';
import { Terminal as TerminalIcon, FileText, Edit3, FilePlus, Play, Search, Brain, Wrench } from 'lucide-react';
import type { ProgressItem } from '@/hooks/useAgentExecution';

interface EmbeddedTerminalPreviewProps {
  output: string;
  elapsedSecs: number;
  progress: ProgressItem[];
  onViewTerminal: () => void;
}

const ACTION_CONFIG: Record<string, { icon: typeof FileText; label: string; color: string }> = {
  reading:   { icon: FileText,  label: '读取文件',  color: 'text-blue-400' },
  editing:   { icon: Edit3,     label: '编辑文件',  color: 'text-amber-400' },
  writing:   { icon: FilePlus,  label: '写入文件',  color: 'text-green-400' },
  running:   { icon: Play,      label: '运行命令',  color: 'text-cyan-400' },
  searching: { icon: Search,    label: '搜索',      color: 'text-purple-400' },
  thinking:  { icon: Brain,     label: '思考中',    color: 'text-emerald-400' },
  tool_use:  { icon: Wrench,    label: '工具调用',  color: 'text-orange-400' },
};

function ActionIcon({ action }: { action: string }) {
  const config = ACTION_CONFIG[action] ?? { icon: TerminalIcon, label: action, color: 'text-white/60' };
  const Icon = config.icon;
  return (
    <div className={`w-5 h-5 flex items-center justify-center flex-shrink-0 ${config.color}`}>
      <Icon className="w-3.5 h-3.5" />
    </div>
  );
}

export const EmbeddedTerminalPreview = memo(function EmbeddedTerminalPreview({
  output,
  elapsedSecs,
  progress,
  onViewTerminal,
}: EmbeddedTerminalPreviewProps) {
  const timelineRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (timelineRef.current) {
      timelineRef.current.scrollTop = timelineRef.current.scrollHeight;
    }
  }, [progress]);

  // 有结构化进度时显示时间线，否则 fallback 到原始输出
  const hasProgress = progress.length > 0;

  return (
    <div className="w-full">
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs text-muted-foreground flex items-center gap-1.5">
          <TerminalIcon className="w-3 h-3" />
          {hasProgress ? `执行中 (${elapsedSecs}s)` : `执行过程 (${elapsedSecs}s)`}
        </span>
        <button
          onClick={onViewTerminal}
          className="text-xs text-emerald-400 hover:text-emerald-300 transition-colors"
        >
          查看完整终端 →
        </button>
      </div>

      <div className="bg-[#1a1a1a] rounded-xl px-3 py-2 max-h-48 overflow-y-auto border border-white/5">
        {hasProgress ? (
          <div ref={timelineRef} className="space-y-1.5">
            {progress.map((item, i) => {
              const config = ACTION_CONFIG[item.action] ?? { label: item.action, color: 'text-white/60' };
              const isLast = i === progress.length - 1;
              return (
                <div key={i} className="flex items-start gap-2">
                  <ActionIcon action={item.action} />
                  <div className="flex-1 min-w-0">
                    <span className={`text-xs font-medium ${isLast ? 'text-white/90' : 'text-white/60'}`}>
                      {config.label}
                    </span>
                    {item.detail && (
                      <p className="text-[11px] text-white/40 truncate">{item.detail}</p>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <pre className="whitespace-pre-wrap font-mono text-xs text-green-400 leading-relaxed">
            {output || '等待输出...'}
          </pre>
        )}
      </div>
    </div>
  );
});
