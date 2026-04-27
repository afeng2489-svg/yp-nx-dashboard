//! Feature Flag Service — 三态切换 + 熔断器逻辑

use chrono::Utc;
use std::sync::Arc;

use super::error::TeamEvolutionError;
use super::feature_flag_repository::SqliteFeatureFlagRepository;
use crate::models::feature_flag::{keys, FeatureFlag, FeatureFlagState};

pub struct FeatureFlagService {
    repo: Arc<SqliteFeatureFlagRepository>,
}

impl FeatureFlagService {
    pub fn new(repo: Arc<SqliteFeatureFlagRepository>) -> Self {
        Self { repo }
    }

    /// Initialize default feature flags if they don't exist
    pub fn initialize_defaults(&self) -> Result<(), TeamEvolutionError> {
        let default_keys = [
            keys::PIPELINE,
            keys::SNAPSHOT,
            keys::CRASH_RESUME,
            keys::FILE_WATCH,
            keys::PROCESS_LIFECYCLE,
        ];

        for key in default_keys {
            if self.repo.find_by_key(key)?.is_none() {
                let now = Utc::now();
                let flag = FeatureFlag {
                    key: key.to_string(),
                    state: FeatureFlagState::On,
                    circuit_breaker: false,
                    error_count: 0,
                    error_threshold: 5,
                    created_at: now,
                    updated_at: now,
                };
                self.repo.upsert(&flag)?;
            }
        }

        Ok(())
    }

    /// Check if a feature is enabled (On + not circuit-broken)
    pub fn is_enabled(&self, key: &str) -> Result<bool, TeamEvolutionError> {
        match self.repo.find_by_key(key)? {
            Some(flag) => Ok(flag.is_enabled()),
            None => Ok(false),
        }
    }

    /// Check if a feature is at least readable (On or ReadOnly, not circuit-broken)
    pub fn is_readable(&self, key: &str) -> Result<bool, TeamEvolutionError> {
        match self.repo.find_by_key(key)? {
            Some(flag) => Ok(flag.is_readable()),
            None => Ok(false),
        }
    }

    /// Guard: return error if feature is disabled
    pub fn require_enabled(&self, key: &str) -> Result<(), TeamEvolutionError> {
        if !self.is_enabled(key)? {
            return Err(TeamEvolutionError::FeatureDisabled(key.to_string()));
        }
        Ok(())
    }

    /// Guard: return error if feature is not readable
    pub fn require_readable(&self, key: &str) -> Result<(), TeamEvolutionError> {
        if !self.is_readable(key)? {
            return Err(TeamEvolutionError::FeatureDisabled(key.to_string()));
        }
        Ok(())
    }

    /// Switch feature flag state
    pub fn set_state(
        &self,
        key: &str,
        state: FeatureFlagState,
    ) -> Result<FeatureFlag, TeamEvolutionError> {
        let mut flag = self
            .repo
            .find_by_key(key)?
            .ok_or_else(|| TeamEvolutionError::FlagNotFound(key.to_string()))?;

        flag.state = state;
        flag.updated_at = Utc::now();
        self.repo.upsert(&flag)?;
        Ok(flag)
    }

    /// Report an error for a feature (increments counter, may trip circuit breaker)
    pub fn report_error(&self, key: &str) -> Result<FeatureFlag, TeamEvolutionError> {
        self.repo.increment_error(key)
    }

    /// Reset circuit breaker and error count
    pub fn reset(&self, key: &str) -> Result<FeatureFlag, TeamEvolutionError> {
        self.repo.reset_error_count(key)?;
        let mut flag = self
            .repo
            .find_by_key(key)?
            .ok_or_else(|| TeamEvolutionError::FlagNotFound(key.to_string()))?;
        flag.state = FeatureFlagState::On;
        flag.updated_at = Utc::now();
        self.repo.upsert(&flag)?;
        Ok(flag)
    }

    /// Get all feature flags
    pub fn list_all(&self) -> Result<Vec<FeatureFlag>, TeamEvolutionError> {
        self.repo.find_all()
    }

    /// Get a single feature flag
    pub fn get(&self, key: &str) -> Result<FeatureFlag, TeamEvolutionError> {
        self.repo
            .find_by_key(key)?
            .ok_or_else(|| TeamEvolutionError::FlagNotFound(key.to_string()))
    }
}
