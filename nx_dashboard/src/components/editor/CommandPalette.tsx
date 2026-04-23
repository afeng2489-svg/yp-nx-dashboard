import { useEffect, useCallback, useState, useMemo, useRef } from 'react';
import { useEditorStore } from '@/stores/editorStore';
import type { WorkflowTemplate } from './types';

interface CommandItem {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  icon: string;
  action: () => void;
  category: 'action' | 'template' | 'node';
}

interface CommandPaletteProps {
  templates: WorkflowTemplate[];
}

export function CommandPalette({ templates }: CommandPaletteProps) {
  const {
    isCommandPaletteOpen,
    setCommandPaletteOpen,
    addNode,
    clearCanvas,
    loadTemplate,
  } = useEditorStore();

  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [position, setPosition] = useState({ x: window.innerWidth / 2 - 288, y: window.innerHeight / 4 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  const [isVisible, setIsVisible] = useState(true);

  const commands: CommandItem[] = useMemo(
    () => [
      {
        id: 'add-agent',
        label: 'Add Agent Node',
        description: 'Add a new agent to the canvas',
        shortcut: 'A',
        icon: '👤',
        action: () => addNode('agent', { x: 250, y: 100 }),
        category: 'node',
      },
      {
        id: 'add-stage',
        label: 'Add Stage Node',
        description: 'Add a new stage to the canvas',
        shortcut: 'S',
        icon: '📦',
        action: () => addNode('stage', { x: 250, y: 100 }),
        category: 'node',
      },
      {
        id: 'add-condition',
        label: 'Add Condition Node',
        description: 'Add a condition to the canvas',
        shortcut: 'C',
        icon: '🔀',
        action: () => addNode('condition', { x: 250, y: 100 }),
        category: 'node',
      },
      {
        id: 'add-loop',
        label: 'Add Loop Node',
        description: 'Add a loop to the canvas',
        shortcut: 'L',
        icon: '🔄',
        action: () => addNode('loop', { x: 250, y: 100 }),
        category: 'node',
      },
      {
        id: 'clear-canvas',
        label: 'Clear Canvas',
        description: 'Remove all nodes and edges',
        shortcut: 'Ctrl+Shift+Delete',
        icon: '🗑️',
        action: () => clearCanvas(),
        category: 'action',
      },
      {
        id: 'export-workflow',
        label: 'Export Workflow',
        description: 'Export workflow as JSON',
        shortcut: 'Ctrl+E',
        icon: '📤',
        action: () => {
          const { exportWorkflow } = useEditorStore.getState();
          const workflow = exportWorkflow();
          const blob = new Blob([JSON.stringify(workflow, null, 2)], {
            type: 'application/json',
          });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = `${workflow.name.replace(/\s+/g, '_')}.json`;
          a.click();
          URL.revokeObjectURL(url);
        },
        category: 'action',
      },
      ...templates.map((t) => ({
        id: `template-${t.id}`,
        label: `Load: ${t.name}`,
        description: t.description,
        icon: '📋',
        action: () => loadTemplate({ nodes: t.nodes, edges: t.edges }),
        category: 'template' as const,
      })),
    ],
    [templates, addNode, clearCanvas, loadTemplate]
  );

  const filteredCommands = useMemo(() => {
    if (!query) return commands;
    const lower = query.toLowerCase();
    return commands.filter(
      (cmd) =>
        cmd.label.toLowerCase().includes(lower) ||
        cmd.description?.toLowerCase().includes(lower)
    );
  }, [commands, query]);

  const executeCommand = useCallback(
    (command: CommandItem) => {
      command.action();
      setCommandPaletteOpen(false);
      setQuery('');
      setSelectedIndex(0);
    },
    [setCommandPaletteOpen]
  );

  const handleHide = useCallback(() => {
    setIsVisible(false);
    setCommandPaletteOpen(false);
    setQuery('');
    setSelectedIndex(0);
  }, [setCommandPaletteOpen]);

  const handleShow = useCallback(() => {
    setIsVisible(true);
    setCommandPaletteOpen(true);
  }, [setCommandPaletteOpen]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('.no-drag')) return;
    setIsDragging(true);
    setDragOffset({
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    });
  }, [position]);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    setPosition({
      x: e.clientX - dragOffset.x,
      y: e.clientY - dragOffset.y,
    });
  }, [isDragging, dragOffset]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  useEffect(() => {
    if (isDragging) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
      return () => {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (isVisible) {
          setCommandPaletteOpen(!isCommandPaletteOpen);
        } else {
          handleShow();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isCommandPaletteOpen, setCommandPaletteOpen, isVisible, handleShow]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!isCommandPaletteOpen || !isVisible) return (
    <button
      onClick={handleShow}
      className="fixed bottom-4 right-4 z-40 p-3 rounded-full bg-card shadow-lg border border-border hover:bg-accent transition-colors"
      title="打开命令面板 (Ctrl+K)"
    >
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
    </button>
  );

  return (
    <div
      className="fixed z-50 bg-card rounded-xl shadow-2xl border border-border overflow-hidden w-full max-w-md"
      style={{ left: position.x, top: position.y }}
    >
      <div
        className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card cursor-move select-none"
        onMouseDown={handleMouseDown}
      >
        <button
          onClick={handleHide}
          className="p-0.5 rounded hover:bg-accent transition-colors no-drag"
          title="隐藏"
        >
          <svg className="w-4 h-4 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
        <svg
          className="w-5 h-5 text-muted-foreground no-drag"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索命令..."
          className="flex-1 bg-transparent outline-none text-sm no-drag"
          autoFocus
        />
        <kbd className="px-2 py-1 text-xs bg-muted rounded border border-border no-drag">ESC</kbd>
      </div>

      <div className="max-h-80 overflow-y-auto p-2">
        {filteredCommands.length === 0 ? (
          <div className="px-4 py-8 text-center text-muted-foreground text-sm">
            没有找到命令
          </div>
        ) : (
          <div className="space-y-1">
            {filteredCommands.map((cmd, index) => (
              <button
                key={cmd.id}
                onClick={() => executeCommand(cmd)}
                onMouseEnter={() => setSelectedIndex(index)}
                className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors ${index === selectedIndex ? 'bg-accent' : 'hover:bg-accent/50'}`}
              >
                <span className="text-lg">{cmd.icon}</span>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium truncate">{cmd.label}</div>
                  {cmd.description && (
                    <div className="text-xs text-muted-foreground truncate">{cmd.description}</div>
                  )}
                </div>
                {cmd.shortcut && (
                  <kbd className="px-2 py-1 text-xs bg-muted rounded border border-border shrink-0">{cmd.shortcut}</kbd>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="px-4 py-2 border-t border-border bg-muted/30">
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">↑↓</kbd>
            导航
          </span>
          <span className="flex items-center gap-1">
            <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">↵</kbd>
            选择
          </span>
          <span className="flex items-center gap-1">
            <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">esc</kbd>
            关闭
          </span>
          <span className="flex items-center gap-1">
            <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">⟳</kbd>
            拖动标题栏
          </span>
        </div>
      </div>
    </div>
  );
}
