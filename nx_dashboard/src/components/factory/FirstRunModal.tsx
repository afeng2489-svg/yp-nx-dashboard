import { FolderOpen, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { setFirstRunChoice } from '@/data/factoryFeatureFlags';

export interface FirstRunModalProps {
  open: boolean;
  onClose: () => void;
  onGreenfield: () => void;
  onExistingCode: () => void;
}

/** AF-UX-01：首启 2 选 1 — 从零 / 已有代码 */
export function FirstRunModal({ open, onGreenfield, onExistingCode, onClose }: FirstRunModalProps) {
  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4"
      data-testid="first-run-modal"
    >
      <div className="w-full max-w-lg rounded-2xl border border-border bg-card shadow-xl p-6 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">欢迎 — 你想怎么开始？</h2>
          <p className="text-sm text-muted-foreground mt-1">
            不需要懂 workflow，选一条路即可在 30 秒内启动第一个 Run。
          </p>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <button
            type="button"
            data-testid="first-run-greenfield"
            className="text-left p-4 rounded-xl border border-border hover:border-primary/40 hover:bg-accent/30 transition-colors"
            onClick={() => {
              setFirstRunChoice('greenfield');
              onGreenfield();
            }}
          >
            <Sparkles className="h-6 w-6 text-primary mb-2" />
            <p className="font-medium text-sm">从零开始</p>
            <p className="text-xs text-muted-foreground mt-1">有个想法，搭可运行骨架</p>
          </button>

          <button
            type="button"
            data-testid="first-run-existing"
            className="text-left p-4 rounded-xl border border-border hover:border-primary/40 hover:bg-accent/30 transition-colors"
            onClick={() => {
              setFirstRunChoice('existing');
              onExistingCode();
            }}
          >
            <FolderOpen className="h-6 w-6 text-primary mb-2" />
            <p className="font-medium text-sm">已有代码</p>
            <p className="text-xs text-muted-foreground mt-1">打开文件夹，直接改代码</p>
          </button>
        </div>

        <div className="flex justify-end">
          <Button type="button" variant="ghost" size="sm" onClick={() => {
            setFirstRunChoice('dismissed');
            onClose();
          }}>
            稍后再说
          </Button>
        </div>
      </div>
    </div>
  );
}
