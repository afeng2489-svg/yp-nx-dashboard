import { useMemo, useCallback } from 'react';
import { FileText, ArrowUp, ArrowDown, Clock, Hash, Search } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SearchResult, SearchHit, SearchMode } from '@/types/search';

// Mode icons
const MODE_ICONS: Record<SearchMode, string> = {
  fts: '📝',
  semantic: '🧠',
  hybrid: '⚡',
};

// Mode labels
const MODE_LABELS: Record<SearchMode, string> = {
  fts: 'Full-Text',
  semantic: 'Semantic',
  hybrid: 'Hybrid',
};

interface SearchResultsProps {
  /** Search results */
  results: SearchResult;
  /** Callback when a result is clicked */
  onResultClick?: (result: SearchHit, file: string, line: number) => void;
  /** Currently selected result index */
  selectedIndex?: number;
  /** Maximum number of results to display */
  maxResults?: number;
  /** Show mode badge on each result */
  showModeBadge?: boolean;
  /** Additional CSS classes */
  className?: string;
}

export function SearchResults({
  results,
  onResultClick,
  selectedIndex = -1,
  maxResults = 50,
  showModeBadge = false,
  className,
}: SearchResultsProps) {
  const displayedResults = useMemo(
    () => results.results.slice(0, maxResults),
    [results.results, maxResults],
  );

  const handleClick = useCallback(
    (result: SearchHit) => {
      onResultClick?.(result, result.file, result.line_number);
    },
    [onResultClick],
  );

  // Group results by file
  const groupedResults = useMemo(() => {
    const groups: Record<string, SearchHit[]> = {};
    for (const result of displayedResults) {
      if (!groups[result.file]) {
        groups[result.file] = [];
      }
      groups[result.file].push(result);
    }
    return groups;
  }, [displayedResults]);

  if (results.results.length === 0) {
    return (
      <div className={cn('flex flex-col items-center justify-center py-12', className)}>
        <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-4">
          <FileText className="w-6 h-6 text-muted-foreground" />
        </div>
        <p className="text-sm text-muted-foreground">No results found</p>
        <p className="text-xs text-muted-foreground mt-1">Try different keywords or search mode</p>
      </div>
    );
  }

  return (
    <div className={cn('space-y-4', className)}>
      {/* Summary Header */}
      <div className="flex items-center justify-between px-2">
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          <span>
            <strong className="text-foreground">{results.total_hits}</strong> results
          </span>
          <span className="flex items-center gap-1">
            <Clock className="w-3.5 h-3.5" />
            {results.search_time_ms}ms
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">Mode:</span>
          <span className="flex items-center gap-1 px-2 py-0.5 bg-secondary rounded text-xs">
            <span>{MODE_ICONS[results.search_mode]}</span>
            <span>{MODE_LABELS[results.search_mode]}</span>
          </span>
        </div>
      </div>

      {/* Results List */}
      <div className="space-y-2">
        {Object.entries(groupedResults).map(([file, hits], groupIndex) => (
          <div key={file} className="border rounded-lg overflow-hidden">
            {/* File Header */}
            <div className="flex items-center gap-2 px-3 py-2 bg-muted/50 border-b">
              <FileText className="w-4 h-4 text-muted-foreground flex-shrink-0" />
              <span className="text-sm font-medium truncate">{file}</span>
              <span className="flex items-center gap-1 ml-auto text-xs text-muted-foreground">
                <Hash className="w-3 h-3" />
                {hits.length}
              </span>
            </div>

            {/* Hits in this file */}
            <div className="divide-y">
              {hits.map((hit, hitIndex) => {
                const globalIndex = displayedResults.findIndex(
                  (r) => r.file === file && r.line_number === hit.line_number,
                );
                const isSelected = globalIndex === selectedIndex;

                return (
                  <button
                    key={`${file}:${hit.line_number}:${hitIndex}`}
                    onClick={() => handleClick(hit)}
                    className={cn(
                      'w-full text-left px-3 py-2 transition-colors',
                      'hover:bg-accent',
                      isSelected && 'bg-accent',
                    )}
                  >
                    <div className="flex items-start gap-2">
                      {/* Line number */}
                      <span className="flex-shrink-0 text-xs text-muted-foreground font-mono w-8">
                        {hit.line_number}
                      </span>

                      {/* Content */}
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-mono truncate">{hit.snippet}</p>
                        <div className="flex items-center gap-2 mt-1">
                          {/* Language badge */}
                          {hit.language && (
                            <span className="text-xs px-1.5 py-0.5 bg-secondary rounded">
                              {hit.language}
                            </span>
                          )}
                          {/* Mode badge */}
                          {showModeBadge && (
                            <span className="text-xs px-1.5 py-0.5 bg-secondary rounded">
                              {MODE_ICONS[hit.search_mode]} {MODE_LABELS[hit.search_mode]}
                            </span>
                          )}
                          {/* Score */}
                          <span className="text-xs text-muted-foreground">
                            Score: {hit.score.toFixed(2)}
                          </span>
                        </div>
                      </div>

                      {/* Navigation hint */}
                      <div className="flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                        {hitIndex > 0 ? (
                          <ArrowUp className="w-4 h-4 text-muted-foreground" />
                        ) : hitIndex < hits.length - 1 ? (
                          <ArrowDown className="w-4 h-4 text-muted-foreground" />
                        ) : null}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>

      {/* Load more hint */}
      {results.results.length > maxResults && (
        <p className="text-center text-sm text-muted-foreground py-2">
          Showing {maxResults} of {results.results.length} results
        </p>
      )}
    </div>
  );
}

// Empty state for no search performed yet
export function SearchResultsEmpty({ className }: { className?: string }) {
  return (
    <div className={cn('flex flex-col items-center justify-center py-12', className)}>
      <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-4">
        <Search className="w-6 h-6 text-muted-foreground" />
      </div>
      <p className="text-sm text-muted-foreground">Start typing to search</p>
      <p className="text-xs text-muted-foreground mt-1">Use Ctrl+Shift+F to focus search</p>
    </div>
  );
}

// Loading state
export function SearchResultsLoading({ className }: { className?: string }) {
  return (
    <div className={cn('flex flex-col items-center justify-center py-12', className)}>
      <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin mb-4" />
      <p className="text-sm text-muted-foreground">Searching...</p>
    </div>
  );
}

// Error state
export function SearchResultsError({ error, className }: { error: string; className?: string }) {
  return (
    <div className={cn('flex flex-col items-center justify-center py-12', className)}>
      <div className="w-12 h-12 rounded-full bg-destructive/10 flex items-center justify-center mb-4">
        <span className="text-destructive text-xl">!</span>
      </div>
      <p className="text-sm text-destructive">Search failed</p>
      <p className="text-xs text-muted-foreground mt-1">{error}</p>
    </div>
  );
}
