import { useEffect, useCallback, useState, useMemo } from 'react';
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
        description: 'Add a new condition to the canvas',
        shortcut: 'C',
        icon: '🔀',
        action: () => addNode('condition', { x: 250, y: 100 }),
        category: 'node',
      },
      {
        id: 'add-loop',
        label: 'Add Loop Node',
        description: 'Add a new loop to the canvas',
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

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setCommandPaletteOpen(!isCommandPaletteOpen);
        return;
      }

      if (!isCommandPaletteOpen) return;

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, filteredCommands.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (filteredCommands[selectedIndex]) {
            executeCommand(filteredCommands[selectedIndex]);
          }
          break;
        case 'Escape':
          e.preventDefault();
          setCommandPaletteOpen(false);
          setQuery('');
          setSelectedIndex(0);
          break;
      }
    },
    [isCommandPaletteOpen, setCommandPaletteOpen, filteredCommands, selectedIndex, executeCommand]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!isCommandPaletteOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={() => {
          setCommandPaletteOpen(false);
          setQuery('');
          setSelectedIndex(0);
        }}
      />

      <div className="relative w-full max-w-lg bg-card rounded-xl shadow-2xl border border-border overflow-hidden">
        <div className="flex items-center gap-3 px-4 py-3 border-b border-border">
          <svg
            className="w-5 h-5 text-muted-foreground"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search commands..."
            className="flex-1 bg-transparent outline-none text-sm"
            autoFocus
          />
          <kbd className="px-2 py-1 text-xs bg-muted rounded border border-border">
            ESC
          </kbd>
        </div>

        <div className="max-h-80 overflow-y-auto p-2">
          {filteredCommands.length === 0 ? (
            <div className="px-4 py-8 text-center text-muted-foreground text-sm">
              No commands found
            </div>
          ) : (
            <div className="space-y-1">
              {filteredCommands.map((cmd, index) => (
                <button
                  key={cmd.id}
                  onClick={() => executeCommand(cmd)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={`
                    w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left
                    transition-colors
                    ${index === selectedIndex ? 'bg-accent' : 'hover:bg-accent/50'}
                  `}
                >
                  <span className="text-lg">{cmd.icon}</span>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium truncate">{cmd.label}</div>
                    {cmd.description && (
                      <div className="text-xs text-muted-foreground truncate">
                        {cmd.description}
                      </div>
                    )}
                  </div>
                  {cmd.shortcut && (
                    <kbd className="px-2 py-1 text-xs bg-muted rounded border border-border shrink-0">
                      {cmd.shortcut}
                    </kbd>
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
              navigate
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">↵</kbd>
              select
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 bg-muted rounded border border-border">esc</kbd>
              close
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}