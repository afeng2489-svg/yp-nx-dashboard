import { useState } from 'react';
import { HelpCircle } from 'lucide-react';
import { ModuleHelpModal } from './ModuleHelpModal';

interface PageHelpButtonProps {
  moduleId: string;
  className?: string;
  floating?: boolean;
}

export function PageHelpButton({ moduleId, className = '', floating = true }: PageHelpButtonProps) {
  const [open, setOpen] = useState(false);

  const base =
    'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg border border-border bg-background hover:bg-accent text-muted-foreground hover:text-foreground transition-all shadow-sm';
  const pos = floating ? 'fixed right-5 bottom-5 z-40 shadow-lg' : '';

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        className={`${base} ${pos} ${className}`}
        title="查看该模块的使用说明"
      >
        <HelpCircle className="w-4 h-4" />
        <span>怎么用</span>
      </button>
      {open && <ModuleHelpModal moduleId={moduleId} onClose={() => setOpen(false)} />}
    </>
  );
}
