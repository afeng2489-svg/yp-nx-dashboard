import { useState, useCallback, useRef, useEffect } from 'react';
import { Search, X, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SearchMode } from '@/types/search';

// Search mode options
const SEARCH_MODES: { value: SearchMode; label: string; icon: string }[] = [
  { value: 'hybrid', label: 'Hybrid', icon: '⚡' },
  { value: 'fts', label: 'Full-Text', icon: '📝' },
  { value: 'semantic', label: 'Semantic', icon: '🧠' },
];

interface SearchBarProps {
  /** Initial search query */
  initialQuery?: string;
  /** Initial search mode */
  initialMode?: SearchMode;
  /** Placeholder text */
  placeholder?: string;
  /** Callback when search is submitted */
  onSearch?: (query: string, mode: SearchMode) => void;
  /** Whether search is loading */
  isLoading?: boolean;
  /** Additional CSS classes */
  className?: string;
  /** Auto-focus on mount */
  autoFocus?: boolean;
}

export function SearchBar({
  initialQuery = '',
  initialMode = 'hybrid',
  placeholder = 'Search code... (Ctrl+Shift+F)',
  onSearch,
  isLoading = false,
  className,
  autoFocus = false,
}: SearchBarProps) {
  const [query, setQuery] = useState(initialQuery);
  const [mode, setMode] = useState<SearchMode>(initialMode);
  const [showModeMenu, setShowModeMenu] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const modeMenuRef = useRef<HTMLDivElement>(null);

  // Handle keyboard shortcut (Ctrl+Shift+F)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'F') {
        e.preventDefault();
        inputRef.current?.focus();
      }
      if (e.key === 'Escape') {
        inputRef.current?.blur();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Close mode menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (modeMenuRef.current && !modeMenuRef.current.contains(e.target as Node)) {
        setShowModeMenu(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (query.trim()) {
        onSearch?.(query.trim(), mode);
      }
    },
    [query, mode, onSearch]
  );

  const handleClear = useCallback(() => {
    setQuery('');
    inputRef.current?.focus();
  }, []);

  const currentMode = SEARCH_MODES.find((m) => m.value === mode) ?? SEARCH_MODES[0];

  return (
    <form onSubmit={handleSubmit} className={cn('w-full', className)}>
      <div className="relative flex items-center">
        {/* Search Icon */}
        <div className="absolute left-3 flex items-center justify-center">
          {isLoading ? (
            <Loader2 className="w-4 h-4 text-muted-foreground animate-spin" />
          ) : (
            <Search className="w-4 h-4 text-muted-foreground" />
          )}
        </div>

        {/* Input */}
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={placeholder}
          autoFocus={autoFocus}
          className={cn(
            'w-full h-10 pl-10 pr-24 py-2 text-sm',
            'bg-background border border-input rounded-lg',
            'placeholder:text-muted-foreground',
            'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
            'disabled:cursor-not-allowed disabled:opacity-50',
            'transition-shadow'
          )}
        />

        {/* Clear Button */}
        {query && (
          <button
            type="button"
            onClick={handleClear}
            className="absolute right-20 flex items-center justify-center w-6 h-6 rounded-md hover:bg-accent transition-colors"
          >
            <X className="w-4 h-4 text-muted-foreground" />
          </button>
        )}

        {/* Mode Selector */}
        <div ref={modeMenuRef} className="absolute right-3">
          <button
            type="button"
            onClick={() => setShowModeMenu(!showModeMenu)}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 rounded-md text-xs font-medium',
              'bg-secondary hover:bg-secondary/80 transition-colors',
              'border border-transparent hover:border-border'
            )}
          >
            <span>{currentMode.icon}</span>
            <span className="text-muted-foreground">{currentMode.label}</span>
          </button>

          {/* Mode Dropdown */}
          {showModeMenu && (
            <div className="absolute top-full right-0 mt-1 py-1 w-40 bg-popover border rounded-lg shadow-lg z-50">
              {SEARCH_MODES.map((searchMode) => (
                <button
                  key={searchMode.value}
                  type="button"
                  onClick={() => {
                    setMode(searchMode.value);
                    setShowModeMenu(false);
                  }}
                  className={cn(
                    'w-full flex items-center gap-2 px-3 py-2 text-sm',
                    'hover:bg-accent transition-colors',
                    mode === searchMode.value && 'bg-accent'
                  )}
                >
                  <span>{searchMode.icon}</span>
                  <span>{searchMode.label}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Keyboard Shortcut Hint */}
      <div className="flex items-center justify-between mt-2 px-1">
        <span className="text-xs text-muted-foreground">
          Press <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Enter</kbd> to search
        </span>
        <span className="text-xs text-muted-foreground">
          <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Esc</kbd> to close
        </span>
      </div>
    </form>
  );
}
