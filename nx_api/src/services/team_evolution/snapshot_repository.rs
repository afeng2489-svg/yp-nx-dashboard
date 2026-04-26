//! 角色快照 + 项目进度 Repository — SQLite 持久化
//!
//! 新表: role_snapshots, role_snapshot_history, project_progress

use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::error::TeamEvolutionError;

// ─── Data Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSnapshot {
    pub id: String,
    pub project_id: String,
    pub team_id: String,
    pub role_id: String,
    pub role_name: String,
    /// idle / thinking / coding / testing / done / failed / hibernated
    pub phase: String,
    pub progress_pct: u32,
    pub current_task: String,
    pub summary: String,
    pub last_cli_output: String,
    pub files_touched: Vec<String>,
    pub execution_count: u32,
    pub checksum: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSnapshotHistory {
    pub id: String,
    pub snapshot_id: String,
    pub project_id: String,
    pub role_id: String,
    pub phase: String,
    pub progress_pct: u32,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProgress {
    pub project_id: String,
    pub team_id: String,
    pub pipeline_id: Option<String>,
    /// idle / requirements_analysis / architecture_design / ... / completed
    pub overall_phase: String,
    pub overall_pct: u32,
    pub total_roles: u32,
    pub active_roles: u32,
    pub completed_roles: u32,
    pub failed_roles: u32,
    pub last_activity: String,
    pub last_activity_at: Option<String>,
    pub updated_at: String,
}

// ─── Repository ───────────────────────────────────────────────

pub struct SqliteSnapshotRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSnapshotRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, TeamEvolutionError> {
        let repo = Self { conn };
        repo.initialize_tables()?;
        Ok(repo)
    }

    fn initialize_tables(&self) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS role_snapshots (
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
            );"
        )?;
        Ok(())
    }

    // ── Role Snapshots ────────────────────────────────────────

    pub fn upsert_snapshot(&self, snap: &RoleSnapshot) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let files_json = serde_json::to_string(&snap.files_touched)
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO role_snapshots
                (id, project_id, team_id, role_id, role_name, phase, progress_pct,
                 current_task, summary, last_cli_output, files_touched, execution_count,
                 checksum, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(project_id, role_id) DO UPDATE SET
                role_name=excluded.role_name, phase=excluded.phase,
                progress_pct=excluded.progress_pct, current_task=excluded.current_task,
                summary=excluded.summary, last_cli_output=excluded.last_cli_output,
                files_touched=excluded.files_touched, execution_count=excluded.execution_count,
                checksum=excluded.checksum, updated_at=excluded.updated_at",
            rusqlite::params![
                snap.id, snap.project_id, snap.team_id, snap.role_id, snap.role_name,
                snap.phase, snap.progress_pct, snap.current_task, snap.summary,
                snap.last_cli_output, files_json, snap.execution_count,
                snap.checksum, snap.created_at, snap.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_snapshots_by_project(&self, project_id: &str) -> Result<Vec<RoleSnapshot>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, team_id, role_id, role_name, phase, progress_pct,
                    current_task, summary, last_cli_output, files_touched, execution_count,
                    checksum, created_at, updated_at
             FROM role_snapshots WHERE project_id = ?1 ORDER BY role_name"
        )?;

        let rows = stmt.query_map(rusqlite::params![project_id], |row| {
            Self::row_to_snapshot(row)
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_snapshot(&self, project_id: &str, role_id: &str) -> Result<Option<RoleSnapshot>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, team_id, role_id, role_name, phase, progress_pct,
                    current_task, summary, last_cli_output, files_touched, execution_count,
                    checksum, created_at, updated_at
             FROM role_snapshots WHERE project_id = ?1 AND role_id = ?2"
        )?;
        let result = stmt.query_row(rusqlite::params![project_id, role_id], |row| {
            Self::row_to_snapshot(row)
        });
        match result {
            Ok(snap) => Ok(Some(snap)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TeamEvolutionError::from(e)),
        }
    }

    // ── History (rotated, keep last 10 per role) ──────────────

    pub fn push_history(&self, snap: &RoleSnapshot) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let hist_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO role_snapshot_history
                (id, snapshot_id, project_id, role_id, phase, progress_pct, summary, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                hist_id, snap.id, snap.project_id, snap.role_id,
                snap.phase, snap.progress_pct, snap.summary, now,
            ],
        )?;

        // Rotate: keep only latest 10
        conn.execute(
            "DELETE FROM role_snapshot_history
             WHERE project_id = ?1 AND role_id = ?2
               AND id NOT IN (
                   SELECT id FROM role_snapshot_history
                   WHERE project_id = ?1 AND role_id = ?2
                   ORDER BY created_at DESC LIMIT 10
               )",
            rusqlite::params![snap.project_id, snap.role_id],
        )?;
        Ok(())
    }

    pub fn find_history(&self, project_id: &str, role_id: &str) -> Result<Vec<RoleSnapshotHistory>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, snapshot_id, project_id, role_id, phase, progress_pct, summary, created_at
             FROM role_snapshot_history
             WHERE project_id = ?1 AND role_id = ?2
             ORDER BY created_at DESC LIMIT 10"
        )?;
        let rows = stmt.query_map(rusqlite::params![project_id, role_id], |row| {
            Ok(RoleSnapshotHistory {
                id: row.get(0)?,
                snapshot_id: row.get(1)?,
                project_id: row.get(2)?,
                role_id: row.get(3)?,
                phase: row.get(4)?,
                progress_pct: row.get(5)?,
                summary: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Project Progress ──────────────────────────────────────

    pub fn upsert_progress(&self, progress: &ProjectProgress) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO project_progress
                (project_id, team_id, pipeline_id, overall_phase, overall_pct,
                 total_roles, active_roles, completed_roles, failed_roles,
                 last_activity, last_activity_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(project_id) DO UPDATE SET
                team_id=excluded.team_id, pipeline_id=excluded.pipeline_id,
                overall_phase=excluded.overall_phase, overall_pct=excluded.overall_pct,
                total_roles=excluded.total_roles, active_roles=excluded.active_roles,
                completed_roles=excluded.completed_roles, failed_roles=excluded.failed_roles,
                last_activity=excluded.last_activity, last_activity_at=excluded.last_activity_at,
                updated_at=excluded.updated_at",
            rusqlite::params![
                progress.project_id, progress.team_id, progress.pipeline_id,
                progress.overall_phase, progress.overall_pct,
                progress.total_roles, progress.active_roles,
                progress.completed_roles, progress.failed_roles,
                progress.last_activity, progress.last_activity_at, progress.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_progress(&self, project_id: &str) -> Result<Option<ProjectProgress>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT project_id, team_id, pipeline_id, overall_phase, overall_pct,
                    total_roles, active_roles, completed_roles, failed_roles,
                    last_activity, last_activity_at, updated_at
             FROM project_progress WHERE project_id = ?1"
        )?;
        let result = stmt.query_row(rusqlite::params![project_id], |row| {
            Ok(ProjectProgress {
                project_id: row.get(0)?,
                team_id: row.get(1)?,
                pipeline_id: row.get(2)?,
                overall_phase: row.get(3)?,
                overall_pct: row.get(4)?,
                total_roles: row.get(5)?,
                active_roles: row.get(6)?,
                completed_roles: row.get(7)?,
                failed_roles: row.get(8)?,
                last_activity: row.get(9)?,
                last_activity_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        });
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TeamEvolutionError::from(e)),
        }
    }

    // ── Helpers ───────────────────────────────────────────────

    fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoleSnapshot> {
        let files_str: String = row.get(10)?;
        let files_touched: Vec<String> = serde_json::from_str(&files_str).unwrap_or_default();

        Ok(RoleSnapshot {
            id: row.get(0)?,
            project_id: row.get(1)?,
            team_id: row.get(2)?,
            role_id: row.get(3)?,
            role_name: row.get(4)?,
            phase: row.get(5)?,
            progress_pct: row.get(6)?,
            current_task: row.get(7)?,
            summary: row.get(8)?,
            last_cli_output: row.get(9)?,
            files_touched,
            execution_count: row.get(11)?,
            checksum: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    }
}
