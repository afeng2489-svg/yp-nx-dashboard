/** Search mode enumeration */
export type SearchMode = 'fts' | 'semantic' | 'hybrid';

/** Symbol context for code navigation */
export interface SymbolContext {
  name: string;
  kind: string;
  signature?: string;
  parent?: string;
}

/** Individual search hit */
export interface SearchHit {
  file: string;
  line_number: number;
  snippet: string;
  score: number;
  search_mode: SearchMode;
  language?: string;
  symbol_context?: SymbolContext;
}

/** Search result container */
export interface SearchResult {
  query: string;
  results: SearchHit[];
  total_hits: number;
  search_time_ms: number;
  search_mode: SearchMode;
  available_modes: SearchMode[];
}

/** Search options */
export interface SearchOptions {
  limit?: number;
  min_score?: number;
  language_filter?: string[];
  path_filter?: string[];
  include_context?: boolean;
  context_lines?: number;
}

/** Index request for reindexing */
export interface IndexRequest {
  workspace_path: string;
  languages?: string[];
  force?: boolean;
}

/** Index response */
export interface IndexResponse {
  documents_indexed: number;
  chunks_indexed: number;
  index_size_bytes: number;
  indexing_time_ms: number;
}

/** Search modes response */
export interface SearchModesResponse {
  modes: SearchModeInfo[];
}

/** Information about a search mode */
export interface SearchModeInfo {
  mode: SearchMode;
  name: string;
  description: string;
  available: boolean;
}
