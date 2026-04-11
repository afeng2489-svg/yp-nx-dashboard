import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Search, Terminal, Plus, Play, Pause, ChevronRight, Clock, X } from 'lucide-react';

// Command types matching the Rust backend
export interface CommandArgument {
  name: string;
  description: string;
  required: boolean;
}

export interface CommandSuggestion {
  command: string;
  description: string;
  arguments: CommandArgument[];
}

export interface CommandHistoryItem {
  command: string;
  timestamp: number;
  success: boolean;
}

// Available commands (mirrors Rust backend)
const AVAILABLE_COMMANDS: CommandSuggestion[] = [
  {
    command: 'issue:new',
    description: 'Create a new issue',
    arguments: [
      { name: 'title', description: 'Issue title', required: true },
      { name: 'description', description: 'Issue description', required: false },
      { name: 'priority', description: 'Priority (low, medium, high, critical)', required: false },
    ],
  },
  {
    command: 'issue:discover',
    description: 'Discover issues in the codebase',
    arguments: [
      { name: 'path', description: 'Path to scan', required: false },
    ],
  },
  {
    command: 'issue:plan',
    description: 'Create a plan for resolving an issue',
    arguments: [
      { name: 'id', description: 'Issue ID (e.g., ISS-001)', required: true },
    ],
  },
  {
    command: 'issue:list',
    description: 'List all issues',
    arguments: [],
  },
  {
    command: 'issue:get',
    description: 'Get issue details',
    arguments: [
      { name: 'id', description: 'Issue ID', required: true },
    ],
  },
  {
    command: 'workflow:execute',
    description: 'Execute a workflow',
    arguments: [
      { name: 'name', description: 'Workflow name', required: true },
      { name: 'vars', description: 'JSON variables', required: false },
    ],
  },
  {
    command: 'workflow:list',
    description: 'List available workflows',
    arguments: [],
  },
  {
    command: 'workflow:status',
    description: 'Show current workflow status',
    arguments: [],
  },
  {
    command: 'workflow:stop',
    description: 'Stop the running workflow',
    arguments: [],
  },
  {
    command: 'session:resume',
    description: 'Resume a paused session',
    arguments: [
      { name: 'key', description: 'Session key', required: true },
    ],
  },
  {
    command: 'session:pause',
    description: 'Pause the current session',
    arguments: [
      { name: 'key', description: 'Session key', required: false },
    ],
  },
  {
    command: 'session:list',
    description: 'List all sessions',
    arguments: [],
  },
  {
    command: 'session:get',
    description: 'Get session details',
    arguments: [
      { name: 'key', description: 'Session key', required: true },
    ],
  },
  {
    command: 'session:delete',
    description: 'Delete a session',
    arguments: [
      { name: 'key', description: 'Session key', required: true },
    ],
  },
];

// Recent commands storage key
const RECENT_COMMANDS_KEY = 'nexusflow_recent_commands';
const MAX_RECENT_COMMANDS = 10;

export function CommandPalette() {
  const [isOpen, setIsOpen] = useState(false);
  const [input, setInput] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentCommands, setRecentCommands] = useState<CommandHistoryItem[]>([]);
  const [showRecent, setShowRecent] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Load recent commands from localStorage
  useEffect(() => {
    const stored = localStorage.getItem(RECENT_COMMANDS_KEY);
    if (stored) {
      try {
        setRecentCommands(JSON.parse(stored));
      } catch {
        // Ignore parse errors
      }
    }
  }, []);

  // Save recent commands to localStorage
  const saveRecentCommand = useCallback((command: string, success: boolean) => {
    setRecentCommands(prev => {
      const newHistory: CommandHistoryItem[] = [
        { command, timestamp: Date.now(), success },
        ...prev.filter(item => item.command !== command),
      ].slice(0, MAX_RECENT_COMMANDS);
      localStorage.setItem(RECENT_COMMANDS_KEY, JSON.stringify(newHistory));
      return newHistory;
    });
  }, []);

  // Filter commands based on input
  const getFilteredCommands = useCallback(() => {
    if (!input.startsWith('/')) {
      return [];
    }
    const query = input.slice(1).toLowerCase();
    if (!query) {
      return AVAILABLE_COMMANDS;
    }
    return AVAILABLE_COMMANDS.filter(
      cmd =>
        cmd.command.toLowerCase().includes(query) ||
        cmd.description.toLowerCase().includes(query)
    );
  }, [input]);

  const filteredCommands = getFilteredCommands();

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Open command palette with Ctrl+K or Cmd+K
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen(true);
        setShowRecent(true);
      }
      // Close with Escape
      if (e.key === 'Escape') {
        setIsOpen(false);
        setInput('');
        setShowRecent(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Focus input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selectedElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedElement) {
        selectedElement.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [selectedIndex]);

  // Handle input changes
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (!value.startsWith('/') && value.length > 0) {
      setInput('/' + value);
    } else {
      setInput(value);
    }
    setSelectedIndex(0);
    setShowRecent(value.length === 0);
  };

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    const items = showRecent ? recentCommands : filteredCommands;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, items.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (items.length > 0) {
          const selected = showRecent
            ? recentCommands[selectedIndex]?.command
            : filteredCommands[selectedIndex]?.command;
          if (selected) {
            executeCommand(selected);
          }
        }
        break;
      case 'Tab':
        e.preventDefault();
        if (filteredCommands.length > 0) {
          setInput('/' + filteredCommands[selectedIndex].command + ' ');
          setShowRecent(false);
        }
        break;
    }
  };

  // Execute a command
  const executeCommand = (command: string) => {
    console.log('Executing command:', command);
    saveRecentCommand(command, true);
    setIsOpen(false);
    setInput('');
    setShowRecent(false);

    // In a real implementation, this would call the backend API
    // For now, we just log the command
    const event = new CustomEvent('nexusflow:command', { detail: { command } });
    window.dispatchEvent(event);
  };

  // Format relative time
  const formatRelativeTime = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  };

  // Get icon for command category
  const getCommandIcon = (command: string) => {
    if (command.startsWith('issue:')) {
      return <Plus className="w-4 h-4 text-blue-400" />;
    }
    if (command.startsWith('workflow:')) {
      return <Play className="w-4 h-4 text-green-400" />;
    }
    if (command.startsWith('session:')) {
      return <Pause className="w-4 h-4 text-yellow-400" />;
    }
    return <Terminal className="w-4 h-4 text-gray-400" />;
  };

  if (!isOpen) {
    return (
      <button
        onClick={() => {
          setIsOpen(true);
          setShowRecent(true);
        }}
        className="fixed bottom-4 right-4 flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-lg border border-gray-700 shadow-lg transition-colors"
      >
        <Terminal className="w-4 h-4" />
        <span className="text-sm">Command Palette</span>
        <kbd className="ml-2 px-1.5 py-0.5 text-xs bg-gray-700 rounded">⌘K</kbd>
      </button>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={() => {
          setIsOpen(false);
          setInput('');
          setShowRecent(false);
        }}
      />

      {/* Command palette */}
      <div className="relative w-full max-w-2xl bg-gray-900 rounded-xl shadow-2xl border border-gray-700 overflow-hidden">
        {/* Search input */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-gray-800">
          <Search className="w-5 h-5 text-gray-500" />
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            placeholder="Type a command (e.g., /issue:new, /workflow:list)"
            className="flex-1 bg-transparent text-gray-100 placeholder-gray-500 outline-none text-lg"
          />
          <button
            onClick={() => {
              setIsOpen(false);
              setInput('');
              setShowRecent(false);
            }}
            className="p-1 hover:bg-gray-800 rounded"
          >
            <X className="w-4 h-4 text-gray-500" />
          </button>
        </div>

        {/* Command list */}
        <div ref={listRef} className="max-h-80 overflow-y-auto">
          {showRecent && recentCommands.length > 0 && (
            <div className="py-2">
              <div className="px-4 py-1 text-xs font-medium text-gray-500 uppercase">
                Recent Commands
              </div>
              {recentCommands.map((item, index) => (
                <button
                  key={item.command + item.timestamp}
                  onClick={() => executeCommand(item.command)}
                  className={`w-full flex items-center gap-3 px-4 py-2 text-left transition-colors ${
                    index === selectedIndex ? 'bg-gray-800' : 'hover:bg-gray-800/50'
                  }`}
                >
                  <Clock className="w-4 h-4 text-gray-500" />
                  <span className="flex-1 text-gray-300">{item.command}</span>
                  <span className="text-xs text-gray-500">
                    {formatRelativeTime(item.timestamp)}
                  </span>
                  {item.success ? (
                    <span className="w-2 h-2 rounded-full bg-green-500" />
                  ) : (
                    <span className="w-2 h-2 rounded-full bg-red-500" />
                  )}
                </button>
              ))}
            </div>
          )}

          {!showRecent && filteredCommands.length > 0 && (
            <div className="py-2">
              <div className="px-4 py-1 text-xs font-medium text-gray-500 uppercase">
                Commands
              </div>
              {filteredCommands.map((cmd, index) => (
                <button
                  key={cmd.command}
                  onClick={() => executeCommand(cmd.command)}
                  className={`w-full flex items-center gap-3 px-4 py-2 text-left transition-colors ${
                    index === selectedIndex ? 'bg-gray-800' : 'hover:bg-gray-800/50'
                  }`}
                >
                  {getCommandIcon(cmd.command)}
                  <div className="flex-1">
                    <div className="text-gray-200">
                      <span className="font-mono text-blue-400">/{cmd.command}</span>
                    </div>
                    <div className="text-sm text-gray-500">{cmd.description}</div>
                  </div>
                  <ChevronRight className="w-4 h-4 text-gray-600" />
                </button>
              ))}
            </div>
          )}

          {!showRecent && input.length > 1 && filteredCommands.length === 0 && (
            <div className="px-4 py-8 text-center text-gray-500">
              No commands found matching "{input.slice(1)}"
            </div>
          )}

          {showRecent && recentCommands.length === 0 && (
            <div className="px-4 py-8 text-center text-gray-500">
              No recent commands. Type "/" to see all commands.
            </div>
          )}
        </div>

        {/* Footer hint */}
        <div className="flex items-center gap-4 px-4 py-2 border-t border-gray-800 text-xs text-gray-500">
          <span>
            <kbd className="px-1.5 py-0.5 bg-gray-800 rounded">↑↓</kbd> Navigate
          </span>
          <span>
            <kbd className="px-1.5 py-0.5 bg-gray-800 rounded">↵</kbd> Execute
          </span>
          <span>
            <kbd className="px-1.5 py-0.5 bg-gray-800 rounded">Tab</kbd> Complete
          </span>
          <span>
            <kbd className="px-1.5 py-0.5 bg-gray-800 rounded">Esc</kbd> Close
          </span>
        </div>
      </div>
    </div>
  );
}

export default CommandPalette;
