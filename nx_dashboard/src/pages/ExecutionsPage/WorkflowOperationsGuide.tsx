import { useState } from 'react';
import { Play, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { WORKFLOW_OPERATIONS } from './constants';

export function WorkflowOperationsGuide() {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <div className="bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5 border border-indigo-500/20 rounded-2xl p-5 relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-r from-indigo-500/5 to-purple-500/5 pointer-events-none" />

      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-indigo-600 hover:text-indigo-700 transition-colors relative"
      >
        <Play className="w-4 h-4" />
        <span>工作流操作说明</span>
        <ChevronRight
          className={cn('w-4 h-4 transition-transform duration-200', isExpanded && 'rotate-90')}
        />
      </button>
      {isExpanded && (
        <div className="mt-4 grid grid-cols-2 md:grid-cols-5 gap-3 relative">
          {WORKFLOW_OPERATIONS.map((op) => (
            <div
              key={op.key}
              className="flex items-start gap-2 p-2 rounded-lg bg-card/50 border border-border/50"
            >
              <span className="flex-shrink-0 w-5 h-5 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 text-white text-xs flex items-center justify-center font-bold shadow-md">
                {op.key}
              </span>
              <div>
                <span className="font-medium text-sm">{op.action}: </span>
                <span className="text-xs text-muted-foreground">{op.desc}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
