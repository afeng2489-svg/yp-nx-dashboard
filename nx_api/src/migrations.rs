use anyhow::Context;
use rusqlite::Connection;

// ── Sessions ──────────────────────────────────────────────────────────────────
pub(crate) const SESSION_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        workflow_id TEXT,
        status TEXT NOT NULL,
        resume_key TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
    CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
    CREATE INDEX IF NOT EXISTS idx_sessions_resume_key ON sessions(resume_key);
";

// ── Workspaces ────────────────────────────────────────────────────────────────
pub(crate) const WORKSPACE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS workspaces (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        owner_id TEXT NOT NULL,
        root_path TEXT,
        settings TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_workspaces_owner ON workspaces(owner_id);
    CREATE INDEX IF NOT EXISTS idx_workspaces_updated_at ON workspaces(updated_at);
";

// ── Workflows ─────────────────────────────────────────────────────────────────
pub(crate) const WORKFLOW_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS workflows (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        version TEXT NOT NULL,
        description TEXT,
        definition TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
";

// ── Executions ────────────────────────────────────────────────────────────────
pub(crate) const EXECUTION_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS executions (
        id            TEXT PRIMARY KEY,
        workflow_id   TEXT NOT NULL,
        status        TEXT NOT NULL DEFAULT 'pending',
        variables     TEXT NOT NULL DEFAULT '{}',
        error         TEXT,
        started_at    TEXT,
        finished_at   TEXT,
        total_tokens  INTEGER NOT NULL DEFAULT 0,
        total_cost_usd REAL NOT NULL DEFAULT 0.0
    );
    CREATE TABLE IF NOT EXISTS stage_results (
        id            TEXT PRIMARY KEY,
        execution_id  TEXT NOT NULL,
        stage_name    TEXT NOT NULL,
        outputs       TEXT NOT NULL DEFAULT '[]',
        quality_gate_result TEXT,
        completed_at  TEXT,
        FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_executions_workflow
        ON executions(workflow_id);
    CREATE INDEX IF NOT EXISTS idx_stage_results_execution
        ON stage_results(execution_id);
";

// ── Teams ─────────────────────────────────────────────────────────────────────
pub(crate) const TEAM_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS teams (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS team_roles (
        id TEXT PRIMARY KEY,
        team_id TEXT,
        name TEXT NOT NULL,
        description TEXT NOT NULL,
        model_config TEXT NOT NULL,
        system_prompt TEXT NOT NULL,
        trigger_keywords TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE SET NULL
    );

    CREATE INDEX IF NOT EXISTS idx_team_roles_team_id ON team_roles(team_id);

    CREATE TABLE IF NOT EXISTS team_role_members (
        team_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        added_at TEXT NOT NULL,
        PRIMARY KEY (team_id, role_id),
        FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
        FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS team_role_skills (
        role_id TEXT NOT NULL,
        skill_id TEXT NOT NULL,
        priority TEXT NOT NULL DEFAULT 'medium',
        PRIMARY KEY (role_id, skill_id),
        FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS team_messages (
        id TEXT PRIMARY KEY,
        team_id TEXT NOT NULL,
        role_id TEXT,
        content TEXT NOT NULL,
        message_type TEXT NOT NULL,
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
        FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE SET NULL
    );

    CREATE INDEX IF NOT EXISTS idx_team_messages_team_id ON team_messages(team_id);
    CREATE INDEX IF NOT EXISTS idx_team_messages_role_id ON team_messages(role_id);
    CREATE INDEX IF NOT EXISTS idx_team_messages_created_at ON team_messages(created_at);

    CREATE TABLE IF NOT EXISTS telegram_bot_configs (
        id TEXT PRIMARY KEY,
        role_id TEXT NOT NULL UNIQUE,
        bot_token TEXT NOT NULL,
        chat_id TEXT,
        enabled INTEGER NOT NULL DEFAULT 0,
        notifications_enabled INTEGER NOT NULL DEFAULT 1,
        conversation_enabled INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
    );
";

// ── Projects ──────────────────────────────────────────────────────────────────
pub(crate) const PROJECT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        team_id TEXT NOT NULL,
        workspace_id TEXT,
        workflow_id TEXT,
        variables TEXT NOT NULL DEFAULT '{}',
        status TEXT NOT NULL DEFAULT 'pending',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_projects_team ON projects(team_id);
    CREATE INDEX IF NOT EXISTS idx_projects_workspace ON projects(workspace_id);
    CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
    CREATE INDEX IF NOT EXISTS idx_projects_updated_at ON projects(updated_at);
";

// ── Project Modules ──────────────────────────────────────────────────────────
pub(crate) const PROJECT_MODULE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS project_modules (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        module_name TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        summary TEXT NOT NULL DEFAULT '',
        files_changed TEXT NOT NULL DEFAULT '[]',
        last_execution_id TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS idx_pm_unique ON project_modules(project_id, module_name);
    CREATE INDEX IF NOT EXISTS idx_pm_project ON project_modules(project_id);
";

// ── Skills ────────────────────────────────────────────────────────────────────
pub(crate) const SKILL_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        category TEXT NOT NULL,
        version TEXT DEFAULT '1.0.0',
        author TEXT,
        tags TEXT DEFAULT '[]',
        parameters TEXT DEFAULT '[]',
        code TEXT,
        is_preset INTEGER DEFAULT 0,
        enabled INTEGER DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_skills_category ON skills(category);
    CREATE INDEX IF NOT EXISTS idx_skills_is_preset ON skills(is_preset);
";

// ── API Keys ──────────────────────────────────────────────────────────────────
pub(crate) const API_KEY_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS api_keys (
        id TEXT PRIMARY KEY,
        provider TEXT NOT NULL UNIQUE,
        encrypted_key TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider);
";

// ── AI Providers ──────────────────────────────────────────────────────────────
pub(crate) const AI_PROVIDER_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_providers (
        id TEXT PRIMARY KEY,
        provider_key TEXT UNIQUE NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        website TEXT,
        encrypted_api_key TEXT,
        base_url TEXT NOT NULL,
        api_format TEXT DEFAULT 'openai',
        auth_field TEXT DEFAULT 'Authorization',
        enabled INTEGER DEFAULT 1,
        config_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS ai_model_mappings (
        id TEXT PRIMARY KEY,
        provider_id TEXT NOT NULL,
        mapping_type TEXT NOT NULL,
        model_id TEXT NOT NULL,
        display_name TEXT,
        config_json TEXT,
        FOREIGN KEY (provider_id) REFERENCES ai_providers(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_model_mappings_provider ON ai_model_mappings(provider_id);
    CREATE INDEX IF NOT EXISTS idx_model_mappings_type ON ai_model_mappings(provider_id, mapping_type);
";

// ── Issues ────────────────────────────────────────────────────────────────────
pub(crate) const ISSUE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS issues (
        id          TEXT PRIMARY KEY,
        title       TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        status      TEXT NOT NULL DEFAULT 'discovered',
        priority    TEXT NOT NULL DEFAULT 'medium',
        perspectives TEXT NOT NULL DEFAULT '[]',
        solution    TEXT,
        depends_on  TEXT NOT NULL DEFAULT '[]',
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
    );
";

// ── Artifacts ─────────────────────────────────────────────────────────────────
pub(crate) const ARTIFACT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS artifacts (
        id            TEXT PRIMARY KEY,
        execution_id  TEXT NOT NULL,
        stage_name    TEXT,
        relative_path TEXT NOT NULL,
        change_type   TEXT NOT NULL,
        size_bytes    INTEGER NOT NULL DEFAULT 0,
        sha256        TEXT,
        mime_type     TEXT,
        created_at    TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_artifacts_execution
        ON artifacts(execution_id);
    CREATE INDEX IF NOT EXISTS idx_artifacts_execution_stage
        ON artifacts(execution_id, stage_name);
";

// ── Group Chat ────────────────────────────────────────────────────────────────
pub(crate) const GROUP_CHAT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS group_sessions (
        id TEXT PRIMARY KEY,
        team_id TEXT NOT NULL,
        name TEXT NOT NULL,
        topic TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        speaking_strategy TEXT NOT NULL DEFAULT 'free',
        consensus_strategy TEXT NOT NULL DEFAULT 'majority',
        moderator_role_id TEXT,
        max_turns INTEGER NOT NULL DEFAULT 10,
        current_turn INTEGER NOT NULL DEFAULT 0,
        turn_policy TEXT NOT NULL DEFAULT 'all',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS group_messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        role_name TEXT NOT NULL,
        content TEXT NOT NULL,
        tool_calls TEXT NOT NULL DEFAULT '[]',
        reply_to TEXT,
        turn_number INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL,
        FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS group_participants (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        role_name TEXT NOT NULL,
        joined_at TEXT NOT NULL,
        last_spoke_at TEXT,
        message_count INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE,
        UNIQUE(session_id, role_id)
    );

    CREATE TABLE IF NOT EXISTS group_conclusions (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL UNIQUE,
        content TEXT NOT NULL,
        consensus_level REAL NOT NULL DEFAULT 0.0,
        participant_scores TEXT NOT NULL DEFAULT '{}',
        agreed_by TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL,
        FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_group_messages_session ON group_messages(session_id);
    CREATE INDEX IF NOT EXISTS idx_group_participants_session ON group_participants(session_id);
";

// ── Wisdom ────────────────────────────────────────────────────────────────────
pub(crate) const WISDOM_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS wisdom (
        id TEXT PRIMARY KEY,
        category TEXT NOT NULL,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        tags TEXT NOT NULL,
        source_session TEXT NOT NULL,
        confidence REAL NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_wisdom_category ON wisdom(category);
    CREATE INDEX IF NOT EXISTS idx_wisdom_created_at ON wisdom(created_at);
    CREATE INDEX IF NOT EXISTS idx_wisdom_confidence ON wisdom(confidence);
";

// ── Feature Flags (team_evolution) ────────────────────────────────────────────
pub(crate) const FEATURE_FLAG_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS feature_flags (
        key TEXT PRIMARY KEY,
        state TEXT NOT NULL DEFAULT 'off',
        circuit_breaker INTEGER NOT NULL DEFAULT 0,
        error_count INTEGER NOT NULL DEFAULT 0,
        error_threshold INTEGER NOT NULL DEFAULT 5,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
";

// ── Pipelines (team_evolution) ────────────────────────────────────────────────
pub(crate) const PIPELINE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS pipelines (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        team_id TEXT NOT NULL,
        current_phase TEXT NOT NULL DEFAULT 'requirements_analysis',
        status TEXT NOT NULL DEFAULT 'idle',
        phase_gate_policy TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_pipelines_project ON pipelines(project_id);

    CREATE TABLE IF NOT EXISTS pipeline_steps (
        id TEXT PRIMARY KEY,
        pipeline_id TEXT NOT NULL,
        task_id TEXT NOT NULL DEFAULT '',
        phase TEXT NOT NULL,
        role_id TEXT NOT NULL,
        instruction TEXT NOT NULL,
        depends_on TEXT NOT NULL DEFAULT '[]',
        status TEXT NOT NULL DEFAULT 'pending',
        output TEXT,
        retry_count INTEGER NOT NULL DEFAULT 0,
        max_retries INTEGER NOT NULL DEFAULT 3,
        created_at TEXT NOT NULL,
        started_at TEXT,
        completed_at TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_steps_pipeline ON pipeline_steps(pipeline_id);
    CREATE INDEX IF NOT EXISTS idx_steps_status ON pipeline_steps(pipeline_id, status);
";

// ── Snapshots (team_evolution) ────────────────────────────────────────────────
pub(crate) const SNAPSHOT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS role_snapshots (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        team_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        role_name TEXT NOT NULL,
        phase TEXT NOT NULL DEFAULT 'idle',
        progress_pct INTEGER DEFAULT 0,
        current_task TEXT DEFAULT '',
        summary TEXT DEFAULT '',
        last_cli_output TEXT DEFAULT '',
        files_touched TEXT DEFAULT '[]',
        execution_count INTEGER DEFAULT 0,
        checksum TEXT DEFAULT '',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS idx_role_snap_unique
        ON role_snapshots(project_id, role_id);

    CREATE TABLE IF NOT EXISTS role_snapshot_history (
        id TEXT PRIMARY KEY,
        snapshot_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        phase TEXT NOT NULL,
        progress_pct INTEGER DEFAULT 0,
        summary TEXT DEFAULT '',
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_snap_hist_role
        ON role_snapshot_history(project_id, role_id);

    CREATE TABLE IF NOT EXISTS project_progress (
        project_id TEXT PRIMARY KEY,
        team_id TEXT NOT NULL,
        pipeline_id TEXT,
        overall_phase TEXT NOT NULL DEFAULT 'idle',
        overall_pct INTEGER DEFAULT 0,
        total_roles INTEGER DEFAULT 0,
        active_roles INTEGER DEFAULT 0,
        completed_roles INTEGER DEFAULT 0,
        failed_roles INTEGER DEFAULT 0,
        last_activity TEXT DEFAULT '',
        last_activity_at TEXT,
        updated_at TEXT NOT NULL
    );
";

// ── Execution Checkpoints (team_evolution resume) ─────────────────────────────
pub(crate) const CHECKPOINT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS execution_checkpoints (
        id TEXT PRIMARY KEY,
        execution_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        pipeline_step_id TEXT,
        role_id TEXT NOT NULL,
        task_prompt TEXT NOT NULL,
        accumulated_output TEXT DEFAULT '',
        phase TEXT NOT NULL DEFAULT 'running',
        started_at TEXT NOT NULL,
        last_heartbeat TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_chk_exec ON execution_checkpoints(execution_id);
    CREATE INDEX IF NOT EXISTS idx_chk_project ON execution_checkpoints(project_id);
    CREATE INDEX IF NOT EXISTS idx_chk_heartbeat ON execution_checkpoints(last_heartbeat);
";

// ── Knowledge Base (RAG) ─────────────────────────────────────────────────────
pub(crate) const KNOWLEDGE_BASE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS knowledge_bases (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        embedding_provider TEXT NOT NULL DEFAULT 'openai',
        embedding_model TEXT NOT NULL DEFAULT 'text-embedding-3-small',
        embedding_dimension INTEGER NOT NULL DEFAULT 1536,
        chunk_size INTEGER NOT NULL DEFAULT 500,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS kb_documents (
        id TEXT PRIMARY KEY,
        knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
        filename TEXT NOT NULL,
        content_type TEXT NOT NULL DEFAULT 'text/markdown',
        file_size INTEGER NOT NULL DEFAULT 0,
        chunk_count INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'pending',
        error TEXT,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_kb_docs_kb_id ON kb_documents(knowledge_base_id);

    CREATE TABLE IF NOT EXISTS kb_chunks (
        id TEXT PRIMARY KEY,
        document_id TEXT NOT NULL REFERENCES kb_documents(id) ON DELETE CASCADE,
        knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
        chunk_index INTEGER NOT NULL,
        content TEXT NOT NULL,
        token_count INTEGER NOT NULL DEFAULT 0,
        embedding BLOB,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_chunks_kb_id ON kb_chunks(knowledge_base_id);
    CREATE INDEX IF NOT EXISTS idx_chunks_doc_id ON kb_chunks(document_id);
";

pub(crate) const EXECUTION_LOG_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS execution_logs (
        id TEXT PRIMARY KEY,
        trace_id TEXT NOT NULL,
        execution_id TEXT NOT NULL,
        stage_name TEXT,
        model TEXT,
        attempt INTEGER NOT NULL DEFAULT 0,
        prompt_tokens INTEGER NOT NULL DEFAULT 0,
        completion_tokens INTEGER NOT NULL DEFAULT 0,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL,
        error TEXT,
        timestamp TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_exec_logs_trace ON execution_logs(trace_id);
    CREATE INDEX IF NOT EXISTS idx_exec_logs_exec ON execution_logs(execution_id);
    CREATE INDEX IF NOT EXISTS idx_exec_logs_status ON execution_logs(status);
";

pub(crate) const ALERT_CONFIG_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS alert_configs (
        id TEXT PRIMARY KEY,
        channel TEXT NOT NULL,
        config_json TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL
    );
";

pub(crate) const SESSION_MESSAGES_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS session_messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        execution_id TEXT,
        role TEXT NOT NULL,
        content_json TEXT NOT NULL,
        pending INTEGER NOT NULL DEFAULT 0,
        responded INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_session_messages_session ON session_messages(session_id);
";

pub(crate) const SPRINT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS sprint_cards (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        priority TEXT NOT NULL DEFAULT 'P2',
        estimated_hours INTEGER NOT NULL DEFAULT 0,
        data_json TEXT NOT NULL DEFAULT '',
        updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS sprint_events (
        id TEXT PRIMARY KEY,
        sprint_id TEXT NOT NULL,
        event_type TEXT NOT NULL,
        detail TEXT,
        created_at TEXT NOT NULL
    );
";

/// All schema migrations in dependency order.
const ALL_SCHEMAS: &[&str] = &[
    SESSION_SCHEMA,
    WORKSPACE_SCHEMA,
    WORKFLOW_SCHEMA,
    EXECUTION_SCHEMA,
    TEAM_SCHEMA,
    PROJECT_SCHEMA,
    PROJECT_MODULE_SCHEMA,
    SKILL_SCHEMA,
    API_KEY_SCHEMA,
    AI_PROVIDER_SCHEMA,
    ISSUE_SCHEMA,
    ARTIFACT_SCHEMA,
    GROUP_CHAT_SCHEMA,
    WISDOM_SCHEMA,
    FEATURE_FLAG_SCHEMA,
    PIPELINE_SCHEMA,
    SNAPSHOT_SCHEMA,
    CHECKPOINT_SCHEMA,
    KNOWLEDGE_BASE_SCHEMA,
    EXECUTION_LOG_SCHEMA,
    ALERT_CONFIG_SCHEMA,
    SPRINT_SCHEMA,
    SESSION_MESSAGES_SCHEMA,
];

/// Column additions for existing tables (ALTER TABLE).
/// These run after ALL_SCHEMAS and use try-catch to be idempotent.
const COLUMN_MIGRATIONS: &[&str] = &[
    // v1.3: 质量门结果
    "ALTER TABLE stage_results ADD COLUMN quality_gate_result TEXT",
    // v1.4: Token/Cost 追踪
    "ALTER TABLE executions ADD COLUMN total_tokens INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE executions ADD COLUMN total_cost_usd REAL NOT NULL DEFAULT 0.0",
];

/// Run all schema migrations against the given database path.
///
/// This opens a dedicated connection, executes all `CREATE TABLE IF NOT EXISTS`
/// and `CREATE INDEX IF NOT EXISTS` statements, and closes the connection.
/// Must be called **before** any repository opens its own connection.
pub fn run_all(db_path: &str) -> anyhow::Result<()> {
    let conn =
        Connection::open(db_path).with_context(|| format!("failed to open DB at {db_path}"))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
        .context("failed to set pragmas")?;

    for (i, schema) in ALL_SCHEMAS.iter().enumerate() {
        conn.execute_batch(schema)
            .with_context(|| format!("migration #{i} failed"))?;
    }

    // Column additions (idempotent — duplicate column is ignored)
    for sql in COLUMN_MIGRATIONS {
        let _ = conn.execute_batch(sql); // ignore "duplicate column" error
    }

    tracing::info!(
        "[Migrations] {} schema migrations executed successfully",
        ALL_SCHEMAS.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_succeeds_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        for schema in ALL_SCHEMAS {
            conn.execute_batch(schema).unwrap();
        }
    }

    #[test]
    fn run_all_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        for schema in ALL_SCHEMAS {
            conn.execute_batch(schema).unwrap();
        }
        // Run again — IF NOT EXISTS makes it safe
        for schema in ALL_SCHEMAS {
            conn.execute_batch(schema).unwrap();
        }
    }
}
