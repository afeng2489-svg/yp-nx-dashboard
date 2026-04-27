//! Pipeline Repository — SQLite 持久化
//!
//! 新表 `pipelines` + `pipeline_steps`，独立于核心表。

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

use super::error::TeamEvolutionError;
use crate::models::pipeline::{
    PhaseGatePolicy, Pipeline, PipelinePhase, PipelineStatus, PipelineStep, StepStatus,
};

pub struct SqlitePipelineRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePipelineRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, TeamEvolutionError> {
        let repo = Self { conn };
        repo.initialize_tables()?;
        Ok(repo)
    }

    fn initialize_tables(&self) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pipelines (
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
            CREATE INDEX IF NOT EXISTS idx_steps_status ON pipeline_steps(pipeline_id, status);",
        )?;
        Ok(())
    }

    // --- Pipeline CRUD ---

    pub fn create_pipeline(&self, pipeline: &Pipeline) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let policy_json = serde_json::to_string(&pipeline.phase_gate_policy)
            .map_err(|e| TeamEvolutionError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO pipelines (id, project_id, team_id, current_phase, status, phase_gate_policy, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                pipeline.id,
                pipeline.project_id,
                pipeline.team_id,
                pipeline.current_phase.as_str(),
                pipeline.status.as_str(),
                policy_json,
                pipeline.created_at.to_rfc3339(),
                pipeline.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn find_pipeline_by_id(&self, id: &str) -> Result<Option<Pipeline>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, team_id, current_phase, status, phase_gate_policy, created_at, updated_at
             FROM pipelines WHERE id = ?1"
        )?;

        let result = stmt.query_row(rusqlite::params![id], |row| Self::row_to_pipeline(row));

        match result {
            Ok(pipeline) => Ok(Some(pipeline)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TeamEvolutionError::from(e)),
        }
    }

    pub fn find_pipeline_by_project(
        &self,
        project_id: &str,
    ) -> Result<Option<Pipeline>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, team_id, current_phase, status, phase_gate_policy, created_at, updated_at
             FROM pipelines WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1"
        )?;

        let result = stmt.query_row(rusqlite::params![project_id], |row| {
            Self::row_to_pipeline(row)
        });

        match result {
            Ok(pipeline) => Ok(Some(pipeline)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TeamEvolutionError::from(e)),
        }
    }

    /// Find all pipelines with Running status
    pub fn find_running_pipelines(&self) -> Result<Vec<Pipeline>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, team_id, current_phase, status, phase_gate_policy, created_at, updated_at
             FROM pipelines WHERE status = 'running'"
        )?;

        let pipelines = stmt
            .query_map([], |row| Self::row_to_pipeline(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(pipelines)
    }

    pub fn update_pipeline_status(
        &self,
        id: &str,
        status: &PipelineStatus,
    ) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE pipelines SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![status.as_str(), now, id],
        )?;
        Ok(())
    }

    pub fn update_pipeline_phase(
        &self,
        id: &str,
        phase: &PipelinePhase,
    ) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE pipelines SET current_phase = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![phase.as_str(), now, id],
        )?;
        Ok(())
    }

    // --- Step CRUD ---

    pub fn create_step(&self, step: &PipelineStep) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let depends_json = serde_json::to_string(&step.depends_on)
            .map_err(|e| TeamEvolutionError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO pipeline_steps (id, pipeline_id, task_id, phase, role_id, instruction, depends_on, status, output, retry_count, max_retries, created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                step.id,
                step.pipeline_id,
                step.task_id,
                step.phase.as_str(),
                step.role_id,
                step.instruction,
                depends_json,
                step.status.as_str(),
                step.output,
                step.retry_count,
                step.max_retries,
                step.created_at.to_rfc3339(),
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn create_steps_batch(&self, steps: &[PipelineStep]) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| TeamEvolutionError::Database(e.to_string()))?;

        for step in steps {
            let depends_json = serde_json::to_string(&step.depends_on)
                .map_err(|e| TeamEvolutionError::Internal(e.to_string()))?;

            tx.execute(
                "INSERT INTO pipeline_steps (id, pipeline_id, task_id, phase, role_id, instruction, depends_on, status, output, retry_count, max_retries, created_at, started_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    step.id,
                    step.pipeline_id,
                    step.task_id,
                    step.phase.as_str(),
                    step.role_id,
                    step.instruction,
                    depends_json,
                    step.status.as_str(),
                    step.output,
                    step.retry_count,
                    step.max_retries,
                    step.created_at.to_rfc3339(),
                    step.started_at.map(|dt| dt.to_rfc3339()),
                    step.completed_at.map(|dt| dt.to_rfc3339()),
                ],
            )?;
        }

        tx.commit()
            .map_err(|e| TeamEvolutionError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn find_steps_by_pipeline(
        &self,
        pipeline_id: &str,
    ) -> Result<Vec<PipelineStep>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, pipeline_id, task_id, phase, role_id, instruction, depends_on, status, output, retry_count, max_retries, created_at, started_at, completed_at
             FROM pipeline_steps WHERE pipeline_id = ?1 ORDER BY created_at"
        )?;

        let steps = stmt
            .query_map(rusqlite::params![pipeline_id], |row| Self::row_to_step(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(steps)
    }

    /// Get all steps that are Ready to execute (dependencies all Completed)
    pub fn get_ready_steps(
        &self,
        pipeline_id: &str,
    ) -> Result<Vec<PipelineStep>, TeamEvolutionError> {
        let all_steps = self.find_steps_by_pipeline(pipeline_id)?;

        let completed_ids: std::collections::HashSet<String> = all_steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| s.id.clone())
            .collect();

        let ready_steps: Vec<PipelineStep> = all_steps
            .into_iter()
            .filter(|s| {
                s.status == StepStatus::Pending
                    && s.depends_on.iter().all(|dep| completed_ids.contains(dep))
            })
            .collect();

        Ok(ready_steps)
    }

    pub fn update_step_status(
        &self,
        step_id: &str,
        status: &StepStatus,
        output: Option<&str>,
    ) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        let started_at = if *status == StepStatus::Running {
            Some(now.clone())
        } else {
            None
        };

        let completed_at = if status.is_terminal() {
            Some(now)
        } else {
            None
        };

        if let Some(started) = &started_at {
            conn.execute(
                "UPDATE pipeline_steps SET status = ?1, output = ?2, started_at = ?3 WHERE id = ?4",
                rusqlite::params![status.as_str(), output, started, step_id],
            )?;
        } else if let Some(completed) = &completed_at {
            conn.execute(
                "UPDATE pipeline_steps SET status = ?1, output = ?2, completed_at = ?3 WHERE id = ?4",
                rusqlite::params![status.as_str(), output, completed, step_id],
            )?;
        } else {
            conn.execute(
                "UPDATE pipeline_steps SET status = ?1, output = ?2 WHERE id = ?3",
                rusqlite::params![status.as_str(), output, step_id],
            )?;
        }

        Ok(())
    }

    pub fn increment_step_retry(&self, step_id: &str) -> Result<u32, TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE pipeline_steps SET retry_count = retry_count + 1, status = 'pending' WHERE id = ?1",
            rusqlite::params![step_id],
        )?;

        let count: u32 = conn.query_row(
            "SELECT retry_count FROM pipeline_steps WHERE id = ?1",
            rusqlite::params![step_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    // --- Helpers ---

    fn row_to_pipeline(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pipeline> {
        let policy_str: String = row.get(5)?;
        let phase_gate_policy: PhaseGatePolicy =
            serde_json::from_str(&policy_str).unwrap_or_default();

        Ok(Pipeline {
            id: row.get(0)?,
            project_id: row.get(1)?,
            team_id: row.get(2)?,
            current_phase: PipelinePhase::from_str(&row.get::<_, String>(3)?)
                .unwrap_or(PipelinePhase::RequirementsAnalysis),
            status: PipelineStatus::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(PipelineStatus::Idle),
            phase_gate_policy,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    fn row_to_step(row: &rusqlite::Row<'_>) -> rusqlite::Result<PipelineStep> {
        let depends_str: String = row.get(6)?;
        let depends_on: Vec<String> = serde_json::from_str(&depends_str).unwrap_or_default();

        let parse_optional_dt = |val: Option<String>| -> Option<chrono::DateTime<Utc>> {
            val.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
        };

        Ok(PipelineStep {
            id: row.get(0)?,
            pipeline_id: row.get(1)?,
            task_id: row.get(2)?,
            phase: PipelinePhase::from_str(&row.get::<_, String>(3)?)
                .unwrap_or(PipelinePhase::RequirementsAnalysis),
            role_id: row.get(4)?,
            instruction: row.get(5)?,
            depends_on,
            status: StepStatus::from_str(&row.get::<_, String>(7)?).unwrap_or(StepStatus::Pending),
            output: row.get(8)?,
            retry_count: row.get(9)?,
            max_retries: row.get(10)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            started_at: parse_optional_dt(row.get(12)?),
            completed_at: parse_optional_dt(row.get(13)?),
        })
    }
}
