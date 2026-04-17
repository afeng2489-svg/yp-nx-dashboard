//! Search API Routes
//!
//! API endpoints for CodexLens Search (FTS5, semantic, hybrid).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::routes::AppState;
use crate::search::{
    FtsSearchHit, FtsSearchResult, FtsQueryType, HybridSearchHit, HybridSearchResult,
    IndexRequest, IndexResponse, SearchHit, SearchModesResponse, SearchModeInfo,
    SearchMode, SearchOptions, SearchResult, SemanticSearchHit, SemanticSearchResult,
    MatchType,
};

/// Application state for search (wrapper for accessing via AppState)
pub struct SearchState {
    /// FTS index (simplified in-memory implementation)
    fts_index: Arc<RwLock<FtsIndex>>,
    /// Semantic index (simplified in-memory implementation)
    semantic_index: Arc<RwLock<SemanticIndex>>,
    /// Hybrid search engine (simplified)
    hybrid_engine: Arc<RwLock<HybridEngine>>,
    /// Workspace path for real file indexing
    workspace_path: Arc<parking_lot::RwLock<Option<String>>>,
}

impl SearchState {
    /// Create new search state with workspace path
    pub fn new(workspace_path: Arc<parking_lot::RwLock<Option<String>>>) -> Self {
        Self {
            fts_index: Arc::new(RwLock::new(FtsIndex::empty())),
            semantic_index: Arc::new(RwLock::new(SemanticIndex::new())),
            hybrid_engine: Arc::new(RwLock::new(HybridEngine::empty())),
            workspace_path,
        }
    }
}



// ============================================================================
// API Handlers
// ============================================================================

/// GET /api/v1/search - Search code
///
/// Query parameters:
/// - q: Search query (required)
/// - mode: Search mode - "fts", "semantic", or "hybrid" (default: "hybrid")
/// - limit: Maximum results (default: 20)
/// - min_score: Minimum score threshold (default: 0.1)
/// - language: Language filter (optional)
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    if query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Query parameter 'q' is required"
            })),
        );
    }

    let mode = params.mode.unwrap_or(SearchMode::Hybrid);
    let options = SearchOptions {
        limit: params.limit,
        min_score: params.min_score,
        language_filter: params.language.map(|l| vec![l]),
        path_filter: None,
        include_context: Some(true),
        context_lines: Some(2),
    };

    let start = std::time::Instant::now();

    let search_state = &state.search_state;
    let results = match mode {
        SearchMode::FTS => {
            let fts_index = search_state.fts_index.read().await;
            search_fts(&fts_index, &query, &options)
        }
        SearchMode::Semantic => {
            let semantic_index = search_state.semantic_index.read().await;
            search_semantic(&semantic_index, &query, &options)
        }
        SearchMode::Hybrid => {
            let hybrid_engine = search_state.hybrid_engine.read().await;
            search_hybrid(&hybrid_engine, &query, &options)
        }
    };

    let search_time_ms = start.elapsed().as_millis() as u64;

    let response = SearchResult {
        query: query.clone(),
        results,
        total_hits: 0, // Will be set by each search implementation
        search_time_ms,
        search_mode: mode,
        available_modes: vec![SearchMode::FTS, SearchMode::Semantic, SearchMode::Hybrid],
    };

    (StatusCode::OK, Json(serde_json::json!(response)))
}

/// POST /api/v1/search/index - Reindex codebase
///
/// Request body: IndexRequest
pub async fn reindex(
    State(state): State<Arc<AppState>>,
    Json(_request): Json<IndexRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Read the workspace path
    let workspace_path = {
        let wp = state.search_state.workspace_path.read();
        wp.clone()
    };

    let workspace = match workspace_path {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IndexResponse {
                    documents_indexed: 0,
                    chunks_indexed: 0,
                    index_size_bytes: 0,
                    indexing_time_ms: 0,
                }),
            );
        }
    };

    // Scan workspace and index files
    let source_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "md"];
    let mut documents_indexed: usize = 0;
    let mut chunks_indexed: usize = 0;
    let mut new_fts_docs = Vec::new();

    if let Ok(entries) = scan_source_files(&workspace, &source_extensions) {
        for (path, content, language) in entries {
            let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            chunks_indexed += lines.len();
            new_fts_docs.push(IndexedDocument {
                id: format!("doc-{}", documents_indexed),
                path,
                content,
                language: Some(language),
                lines,
            });
            documents_indexed += 1;
        }
    }

    let index_size_bytes = new_fts_docs.iter()
        .map(|d| d.content.len())
        .sum::<usize>();

    // Update FTS index
    {
        let mut fts_index = state.search_state.fts_index.write().await;
        fts_index.documents = new_fts_docs.clone();
    }

    // Update hybrid engine's FTS index
    {
        let mut hybrid = state.search_state.hybrid_engine.write().await;
        hybrid.fts_index.documents = new_fts_docs;
    }

    let response = IndexResponse {
        documents_indexed,
        chunks_indexed,
        index_size_bytes,
        indexing_time_ms: start.elapsed().as_millis() as u64,
    };

    (StatusCode::OK, Json(response))
}

/// GET /api/v1/search/modes - Get available search modes
pub async fn get_search_modes() -> impl IntoResponse {
    let response = SearchModesResponse::available_modes();
    (StatusCode::OK, Json(response))
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct SearchParams {
    /// Search query
    pub q: Option<String>,
    /// Search mode
    pub mode: Option<SearchMode>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Minimum score
    pub min_score: Option<f32>,
    /// Language filter
    pub language: Option<String>,
}

// ============================================================================
// Simplified In-Memory Index Implementations
// ============================================================================

/// Simplified FTS index
struct FtsIndex {
    documents: Vec<IndexedDocument>,
}

#[derive(Clone)]
struct IndexedDocument {
    id: String,
    path: String,
    content: String,
    language: Option<String>,
    lines: Vec<String>,
}

impl FtsIndex {
    fn empty() -> Self {
        Self { documents: Vec::new() }
    }

    fn search(&self, query: &str, _options: &SearchOptions) -> Vec<FtsSearchHit> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for doc in &self.documents {
            for (line_num, line) in doc.lines.iter().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    let score = if line.to_lowercase().starts_with(&query_lower) {
                        1.0
                    } else {
                        0.5
                    };

                    results.push(FtsSearchHit {
                        document_id: doc.id.clone(),
                        file: doc.path.clone(),
                        line_number: line_num + 1,
                        snippet: line.clone(),
                        score,
                        matched_terms: vec![query.to_string()],
                        language: doc.language.clone(),
                    });
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for FtsIndex {
    fn default() -> Self {
        Self::empty()
    }
}

/// Simplified semantic index
struct SemanticIndex {
    documents: Vec<SemanticDocument>,
}

struct SemanticDocument {
    id: String,
    path: String,
    chunks: Vec<SemanticChunk>,
    language: Option<String>,
}

struct SemanticChunk {
    id: String,
    content: String,
    start_line: usize,
    end_line: usize,
    vector: Vec<f32>,
}

impl SemanticIndex {
    fn new() -> Self {
        Self { documents: Vec::new() }
    }

    fn search(&self, query: &str, options: &SearchOptions) -> Vec<SemanticSearchHit> {
        // Simplified: generate mock embeddings and return results
        let limit = options.limit.unwrap_or(20);

        let mut results = Vec::new();
        for doc in &self.documents {
            for chunk in &doc.chunks {
                // In real implementation, would compute cosine similarity
                // For mock, just check if query appears in content
                if chunk.content.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(SemanticSearchHit {
                        chunk_id: chunk.id.clone(),
                        document_id: doc.id.clone(),
                        file: doc.path.clone(),
                        start_line: chunk.start_line,
                        end_line: chunk.end_line,
                        snippet: chunk.content.clone(),
                        semantic_score: 0.85,
                        relevance_score: Some(0.85),
                        language: doc.language.clone(),
                        symbol_context: None,
                    });
                }
            }
        }

        results.truncate(limit);
        results
    }
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Simplified hybrid engine
struct HybridEngine {
    fts_index: FtsIndex,
    semantic_index: SemanticIndex,
}

impl HybridEngine {
    fn empty() -> Self {
        Self {
            fts_index: FtsIndex::empty(),
            semantic_index: SemanticIndex::new(),
        }
    }

    fn search(&self, query: &str, options: &SearchOptions) -> Vec<HybridSearchHit> {
        let fts_results = self.fts_index.search(query, options);
        let semantic_results = self.semantic_index.search(query, options);

        let mut hit_map = std::collections::HashMap::new();

        // Process FTS results
        for hit in fts_results {
            hit_map.insert(hit.document_id.clone(), HybridSearchHit {
                document_id: hit.document_id.clone(),
                file: hit.file.clone(),
                snippet: hit.snippet.clone(),
                start_line: hit.line_number,
                end_line: hit.line_number,
                score: hit.score * 0.5, // Keyword weight
                semantic_score: 0.0,
                keyword_score: hit.score,
                language: hit.language.clone(),
                match_types: vec![],
            });
        }

        // Process semantic results
        for hit in semantic_results {
            let score = hit.semantic_score * 0.5; // Semantic weight
            if let Some(existing) = hit_map.get_mut(&hit.document_id) {
                existing.score += score;
                existing.semantic_score = hit.semantic_score;
                existing.match_types.push(MatchType::Semantic);
            } else {
                hit_map.insert(hit.document_id.clone(), HybridSearchHit {
                    document_id: hit.document_id.clone(),
                    file: hit.file.clone(),
                    snippet: hit.snippet.clone(),
                    start_line: hit.start_line,
                    end_line: hit.end_line,
                    score,
                    semantic_score: hit.semantic_score,
                    keyword_score: 0.0,
                    language: hit.language.clone(),
                    match_types: vec![MatchType::Semantic],
                });
            }
        }

        let mut results: Vec<HybridSearchHit> = hit_map.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(options.limit.unwrap_or(20));
        results
    }
}

impl Default for HybridEngine {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// File Scanning Helper
// ============================================================================

/// Scan workspace for source files and return (path, content, language) tuples
fn scan_source_files(
    workspace: &str,
    extensions: &[&str],
) -> Result<Vec<(String, String, String)>, std::io::Error> {
    let mut results = Vec::new();
    scan_dir_recursive(std::path::Path::new(workspace), extensions, &mut results)?;
    Ok(results)
}

fn scan_dir_recursive(
    dir: &std::path::Path,
    extensions: &[&str],
    results: &mut Vec<(String, String, String)>,
) -> Result<(), std::io::Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs and common non-source dirs
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == "dist" || name == "build" {
            continue;
        }

        if path.is_dir() {
            scan_dir_recursive(&path, extensions, results)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                let language = match ext {
                    "rs" => "Rust",
                    "ts" | "tsx" => "TypeScript",
                    "js" | "jsx" => "JavaScript",
                    "py" => "Python",
                    "go" => "Go",
                    "java" => "Java",
                    "md" => "Markdown",
                    _ => "Unknown",
                };
                // Limit file size to 100KB
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.len() <= 100_000 {
                        results.push((
                            path.to_string_lossy().to_string(),
                            content,
                            language.to_string(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// Search Implementation Helpers
// ============================================================================

fn search_fts(index: &FtsIndex, query: &str, options: &SearchOptions) -> Vec<SearchHit> {
    index.search(query, options)
        .into_iter()
        .map(|h| h.into())
        .collect()
}

fn search_semantic(index: &SemanticIndex, query: &str, options: &SearchOptions) -> Vec<SearchHit> {
    index.search(query, options)
        .into_iter()
        .map(|h| h.into())
        .collect()
}

fn search_hybrid(engine: &HybridEngine, query: &str, options: &SearchOptions) -> Vec<SearchHit> {
    engine.search(query, options)
        .into_iter()
        .map(|h| h.into())
        .collect()
}
