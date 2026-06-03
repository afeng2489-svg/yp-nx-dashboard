import { Bot, User } from 'lucide-react';
import { useSettingsStore } from '@/stores/settingsStore';
import { cn } from '@/lib/utils';

export type FactoryEditMode = 'agent' | 'human';

export interface EditModeToggleProps {
  /** Show only when a Run is active */
  active?: boolean;
  className?: string;
}

/** AF-UX-12 Beta: Agent 改 / 我来改 */
export function EditModeToggle({ active, className }: EditModeToggleProps) {
  const mode = useSettingsStore((s) => s.factoryEditMode);
  const setMode = useSettingsStore((s) => s.setFactoryEditMode);

  if (!active) return null;

  return (
    <div
      className={cn(
        'inline-flex items-center rounded-lg border border-border/60 bg-muted/30 p-0.5 text-xs',
        className,
      )}
      data-testid="edit-mode-toggle"
    >
      <button
        type="button"
        className={cn(
          'flex items-center gap-1 rounded-md px-2.5 py-1 transition-colors',
          mode === 'agent' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground',
        )}
        onClick={() => setMode('agent')}
        title="Agent 自动改代码"
      >
        <Bot className="h-3.5 w-3.5" />
        Agent 改
      </button>
      <button
        type="button"
        className={cn(
          'flex items-center gap-1 rounded-md px-2.5 py-1 transition-colors',
          mode === 'human' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground',
        )}
        onClick={() => setMode('human')}
        title="暂停 Agent 写入，使用文件树手工编辑"
      >
        <User className="h-3.5 w-3.5" />
        我来改
      </button>
    </div>
  );
}

export function HumanEditHint() {
  const mode = useSettingsStore((s) => s.factoryEditMode);
  if (mode !== 'human') return null;
  return (
    <p className="text-xs text-muted-foreground rounded-md border border-border/50 bg-muted/20 px-3 py-2">
      手工模式：使用左侧文件树编辑；完成后在输入框追加指令或点「继续 Run」让 Agent 基于你的改动继续。
    </p>
  );
}
