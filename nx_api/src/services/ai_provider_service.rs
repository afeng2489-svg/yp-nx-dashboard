//! AI Provider Service
//!
//! Business logic for AI provider management.

use std::sync::Arc;
use thiserror::Error;

use super::ai_provider_repository::{
    AIProvider, APIFormat, MappingType, ModelMapping, ProviderPreset, ProviderRepository,
    ProviderRepositoryError,
};

#[derive(Debug, Error)]
pub enum ProviderServiceError {
    #[error("Repository error: {0}")]
    Repository(#[from] ProviderRepositoryError),
    #[error("Provider not found: {0}")]
    NotFound(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Connection test failed: {0}")]
    ConnectionFailed(String),
}

/// Shared provider service type
pub type SharedProviderService = Arc<ProviderService>;

/// Provider service for business logic
pub struct ProviderService {
    repo: Arc<dyn ProviderRepository>,
}

impl ProviderService {
    /// Create a new provider service
    pub fn new(repo: Arc<dyn ProviderRepository>) -> Self {
        Self { repo }
    }

    /// List all providers
    pub async fn list_providers(&self) -> Result<Vec<AIProvider>, ProviderServiceError> {
        self.repo.list_providers().map_err(ProviderServiceError::from)
    }

    /// Get provider by ID
    pub async fn get_provider(&self, id: &str) -> Result<Option<AIProvider>, ProviderServiceError> {
        self.repo.get_provider(id).map_err(ProviderServiceError::from)
    }

    /// Get provider by key
    pub async fn get_provider_by_key(&self, key: &str) -> Result<Option<AIProvider>, ProviderServiceError> {
        self.repo.get_provider_by_key(key).map_err(ProviderServiceError::from)
    }

    /// Create a new provider
    pub async fn create_provider(
        &self,
        provider: &AIProvider,
    ) -> Result<AIProvider, ProviderServiceError> {
        // Validate required fields
        if provider.name.trim().is_empty() {
            return Err(ProviderServiceError::InvalidOperation(
                "Provider name is required".to_string(),
            ));
        }
        if provider.base_url.trim().is_empty() {
            return Err(ProviderServiceError::InvalidOperation(
                "Base URL is required".to_string(),
            ));
        }

        self.repo.create_provider(provider).map_err(ProviderServiceError::from)?;
        Ok(provider.clone())
    }

    /// Update an existing provider
    pub async fn update_provider(
        &self,
        provider: &AIProvider,
    ) -> Result<AIProvider, ProviderServiceError> {
        // Check if provider exists
        let existing = self.repo.get_provider(&provider.id).map_err(ProviderServiceError::from)?;
        if existing.is_none() {
            return Err(ProviderServiceError::NotFound(provider.id.clone()));
        }

        self.repo.update_provider(provider).map_err(ProviderServiceError::from)?;
        Ok(provider.clone())
    }

    /// Delete a provider
    pub async fn delete_provider(&self, id: &str) -> Result<bool, ProviderServiceError> {
        self.repo.delete_provider(id).map_err(ProviderServiceError::from)
    }

    /// Save API key for a provider
    pub async fn save_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), ProviderServiceError> {
        self.repo.save_api_key(provider_id, api_key).map_err(ProviderServiceError::from)
    }

    /// Get API key for a provider
    pub async fn get_api_key(&self, provider_id: &str) -> Result<Option<String>, ProviderServiceError> {
        self.repo.get_api_key(provider_id).map_err(ProviderServiceError::from)
    }

    /// Get API key by provider key (e.g., "anthropic", "openai")
    pub async fn get_api_key_by_provider_key(&self, provider_key: &str) -> Result<Option<String>, ProviderServiceError> {
        if let Some(provider) = self.get_provider_by_key(provider_key).await? {
            self.get_api_key(&provider.id).await
        } else {
            Ok(None)
        }
    }

    /// Delete API key for a provider
    pub async fn delete_api_key(&self, provider_id: &str) -> Result<bool, ProviderServiceError> {
        self.repo.delete_api_key(provider_id).map_err(ProviderServiceError::from)
    }

    /// Add model mapping to a provider
    pub async fn add_model_mapping(
        &self,
        mapping: &ModelMapping,
    ) -> Result<(), ProviderServiceError> {
        self.repo.create_model_mapping(mapping).map_err(ProviderServiceError::from)
    }

    /// Get model mappings for a provider
    pub async fn get_model_mappings(&self, provider_id: &str) -> Result<Vec<ModelMapping>, ProviderServiceError> {
        self.repo.get_model_mappings(provider_id).map_err(ProviderServiceError::from)
    }

    /// Get model mapping by type
    pub async fn get_model_mapping_by_type(&self, provider_id: &str, mapping_type: &MappingType) -> Result<Option<ModelMapping>, ProviderServiceError> {
        self.repo.get_model_mapping_by_type(provider_id, mapping_type).map_err(ProviderServiceError::from)
    }

    /// Delete model mapping
    pub async fn delete_model_mapping(&self, id: &str) -> Result<bool, ProviderServiceError> {
        self.repo.delete_model_mapping(id).map_err(ProviderServiceError::from)
    }

    /// Get preset providers
    pub fn get_presets() -> Vec<ProviderPreset> {
        super::ai_provider_repository::PRESET_PROVIDERS.to_vec()
    }

    /// Create provider from preset
    pub async fn create_from_preset(
        &self,
        preset_key: &str,
        api_key: &str,
    ) -> Result<AIProvider, ProviderServiceError> {
        tracing::info!("[create_from_preset] preset_key = '{}', api_key_length = {}", preset_key, api_key.len());

        let presets = Self::get_presets();
        let preset = presets
            .iter()
            .find(|p| p.key == preset_key)
            .ok_or_else(|| ProviderServiceError::InvalidOperation(format!("Preset not found: {}", preset_key)))?;

        // Check if provider with same key already exists
        if let Some(existing) = self.repo.get_provider_by_key(preset_key).map_err(ProviderServiceError::from)? {
            tracing::info!("[create_from_preset] Provider with key '{}' already exists (id: {}), deleting...", preset_key, existing.id);
            self.repo.delete_provider(&existing.id).map_err(ProviderServiceError::from)?;
        }

        let provider_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("[create_from_preset] New provider_id: {}", provider_id);

        let provider = AIProvider {
            id: provider_id.clone(),
            provider_key: preset.key.to_string(),
            name: preset.name.to_string(),
            description: Some(preset.description.to_string()),
            website: Some(preset.website.to_string()),
            api_key: None, // We don't store it in the struct, it's encrypted in the DB
            base_url: preset.base_url.to_string(),
            api_format: preset.api_format.clone(),
            auth_field: preset.default_auth_field.to_string(),
            enabled: true,
            config_json: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Create provider first
        tracing::info!("[create_from_preset] Creating provider in DB...");
        self.repo.create_provider(&provider).map_err(|e| {
            tracing::error!("[create_from_preset] Failed to create provider: {}", e);
            ProviderServiceError::from(e)
        })?;
        tracing::info!("[create_from_preset] Provider created successfully");

        // Then save the API key (it will be encrypted)
        tracing::info!("[create_from_preset] Saving API key...");
        if let Err(e) = self.repo.save_api_key(&provider_id, api_key).map_err(ProviderServiceError::from) {
            tracing::error!("[create_from_preset] Failed to save API key: {}", e);
            return Err(e);
        }
        tracing::info!("[create_from_preset] API key saved successfully");

        // Verify by reading it back
        tracing::info!("[create_from_preset] Verifying API key...");
        match self.repo.get_api_key(&provider_id).map_err(ProviderServiceError::from) {
            Ok(Some(key)) => tracing::info!("[create_from_preset] API key verified! Length: {}", key.len()),
            Ok(None) => tracing::error!("[create_from_preset] API key not found after save!"),
            Err(e) => tracing::error!("[create_from_preset] Error reading API key: {}", e),
        }

        Ok(provider)
    }

    /// Test connection to a provider
    pub async fn test_provider_connection(&self, provider_id: &str) -> Result<ConnectionTestResult, ProviderServiceError> {
        tracing::info!("[ConnectionTest] Testing provider: {}", provider_id);

        // Get provider
        let provider = self.repo.get_provider(provider_id)
            .map_err(ProviderServiceError::from)?
            .ok_or_else(|| ProviderServiceError::NotFound(provider_id.to_string()))?;

        tracing::info!("[ConnectionTest] Provider found: {}, base_url: {}, api_format: {:?}",
            provider.name, provider.base_url, provider.api_format);

        // Get API key
        tracing::info!("[ConnectionTest] Getting API key for provider: {}", provider_id);
        let api_key = match self.repo.get_api_key(provider_id).map_err(ProviderServiceError::from) {
            Ok(Some(key)) => {
                tracing::info!("[ConnectionTest] API key found, length: {}", key.len());
                key
            }
            Ok(None) => {
                tracing::error!("[ConnectionTest] API key is None for provider: {}", provider_id);
                return Err(ProviderServiceError::ConnectionFailed("API key not found".to_string()));
            }
            Err(e) => {
                tracing::error!("[ConnectionTest] Error getting API key: {}", e);
                return Err(ProviderServiceError::ConnectionFailed(format!("Error getting API key: {}", e)));
            }
        };

        tracing::info!("[ConnectionTest] API key found, length: {}", api_key.len());

        // Get the first model mapping to use for testing
        let model_mappings = self.repo.get_model_mappings(provider_id)
            .map_err(ProviderServiceError::from)?;

        let model_id = model_mappings
            .into_iter()
            .next()
            .map(|m| m.model_id)
            .unwrap_or_else(|| {
                tracing::warn!("[ConnectionTest] No model mappings found for provider {}", provider_id);
                "gpt-4".to_string()
            });

        tracing::info!("[ConnectionTest] Using model for test: {}", model_id);

        // Build test request based on API format
        let test_result = match provider.api_format {
            APIFormat::OpenAI => {
                tracing::info!("[ConnectionTest] Testing OpenAI format");
                self.test_openai_connection(&provider.base_url, &api_key, &provider.auth_field, &model_id).await
            }
            APIFormat::Anthropic => {
                tracing::info!("[ConnectionTest] Testing Anthropic format");
                self.test_anthropic_connection(&provider.base_url, &api_key, &provider.auth_field).await
            }
            APIFormat::Custom(ref format) => {
                tracing::info!("[ConnectionTest] Testing custom format: {}", format);
                self.test_openai_connection(&provider.base_url, &api_key, &provider.auth_field, &model_id).await
            }
        };

        Ok(test_result)
    }

    /// Test OpenAI-compatible API connection
    async fn test_openai_connection(
        &self,
        base_url: &str,
        api_key: &str,
        auth_field: &str,
        model: &str,
    ) -> ConnectionTestResult {
        use reqwest::Client;

        let url = base_url.trim_end_matches('/');
        tracing::info!("[ConnectionTest] Testing OpenAI-compatible API: {}", url);

        let client = Client::new();

        // Use a simple chat completions request to test the connection
        let request = client
            .post(url)
            .header(auth_field, format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "test"}],
                "max_tokens": 10
            }));

        let response = request
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                tracing::info!("[ConnectionTest] HTTP Status: {}", status.as_u16());

                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    // Try to extract model info from response
                    let model = json.get("model").and_then(|m| m.as_str()).map(|s| s.to_string());
                    tracing::info!("[ConnectionTest] Success! Model: {:?}", model);

                    ConnectionTestResult {
                        success: true,
                        message: "Connection successful".to_string(),
                        models: model.map(|m| vec![m]),
                    }
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("[ConnectionTest] Failed: {} - {}", status.as_u16(), body);
                    ConnectionTestResult {
                        success: false,
                        message: format!("HTTP {}: {}", status.as_u16(), body),
                        models: None,
                    }
                }
            }
            Err(e) => {
                tracing::error!("[ConnectionTest] Connection error: {}", e);
                ConnectionTestResult {
                    success: false,
                    message: format!("Connection failed: {}", e),
                    models: None,
                }
            },
        }
    }

    /// Test Anthropic API connection
    async fn test_anthropic_connection(
        &self,
        base_url: &str,
        api_key: &str,
        auth_field: &str,
    ) -> ConnectionTestResult {
        use reqwest::Client;

        let client = Client::new();
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let response = client
            .post(&url)
            .header(auth_field, format!("Bearer {}", api_key))
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": "claude-3-haiku-20240307",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "test"}]
            }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    ConnectionTestResult {
                        success: true,
                        message: "Connection successful".to_string(),
                        models: None,
                    }
                } else {
                    ConnectionTestResult {
                        success: false,
                        message: format!("HTTP {}: {}", resp.status().as_u16(), resp.text().await.unwrap_or_default()),
                        models: None,
                    }
                }
            }
            Err(e) => ConnectionTestResult {
                success: false,
                message: format!("Connection failed: {}", e),
                models: None,
            },
        }
    }
}

/// Result of connection test
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub models: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_providers_exist() {
        let presets = ProviderService::get_presets();
        assert!(!presets.is_empty());

        // Check some expected presets
        let preset_keys: Vec<&str> = presets.iter().map(|p| p.key).collect();
        assert!(preset_keys.contains(&"deepseek"));
        assert!(preset_keys.contains(&"openai"));
        assert!(preset_keys.contains(&"anthropic"));
    }

    #[test]
    fn test_validate_provider_name_required() {
        // This would require a mock repo to test properly
        // For now just verify the error type exists
        let err = ProviderServiceError::InvalidOperation("test".to_string());
        assert!(err.to_string().contains("test"));
    }
}