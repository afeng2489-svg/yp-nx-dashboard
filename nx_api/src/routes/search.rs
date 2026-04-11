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
}

impl SearchState {
    /// Create new search state
    pub fn new() -> Self {
        Self {
            fts_index: Arc::new(RwLock::new(FtsIndex::new())),
            semantic_index: Arc::new(RwLock::new(SemanticIndex::new())),
            hybrid_engine: Arc::new(RwLock::new(HybridEngine::new())),
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
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
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<IndexRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Simulate indexing documents
    let documents_indexed = 42;
    let chunks_indexed = 156;
    let index_size_bytes = 1024 * 512;

    // In a real implementation, this would:
    // 1. Walk the workspace directory
    // 2. Parse files by language
    // 3. Extract code symbols and chunks
    // 4. Generate embeddings
    // 5. Build FTS and vector indices

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

struct IndexedDocument {
    id: String,
    path: String,
    content: String,
    language: Option<String>,
    lines: Vec<String>,
}

impl FtsIndex {
    fn new() -> Self {
        let mut index = Self { documents: Vec::new() };
        // Add some sample documents for demonstration
        index.add_sample_documents();
        index
    }

    fn add_sample_documents(&mut self) {
        let samples = vec![
            ("1", "src/auth/login.ts", "export async function login(email: string, password: string) {\n  const user = await db.users.findByEmail(email);\n  if (!user) throw new Error('Invalid credentials');\n  return user;\n}", Some("TypeScript".to_string())),
            ("2", "src/auth/logout.ts", "export async function logout(sessionId: string) {\n  await sessions.delete(sessionId);\n  return { success: true };\n}", Some("TypeScript".to_string())),
            ("3", "src/api/users.rs", "pub fn get_user(id: UserId) -> Result<User, Error> {\n    users.find_by_id(id)\n}", Some("Rust".to_string())),
            ("4", "src/api/users.rs", "pub async fn create_user(name: String, email: String) -> User {\n    User { id: generate_id(), name, email }\n}", Some("Rust".to_string())),
            ("5", "src/components/Button.tsx", "export function Button({ children, onClick }: ButtonProps) {\n  return <button onClick={onClick} className=\"btn\">{children}</button>;\n}", Some("TypeScript".to_string())),
        ];

        for (id, path, content, language) in samples {
            let content_str = content.to_string();
            let lines: Vec<String> = content_str.lines().map(|l| l.to_string()).collect();
            self.documents.push(IndexedDocument {
                id: id.to_string(),
                path: path.to_string(),
                content: content_str,
                language,
                lines,
            });
        }
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
        Self::new()
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
    fn new() -> Self {
        Self {
            fts_index: FtsIndex::new(),
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
        Self::new()
    }
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
