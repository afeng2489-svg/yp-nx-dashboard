//! Telegram service
//!
//! Telegram Bot API integration for notifications and bidirectional conversation.

use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

use crate::models::team::{TelegramBotConfig, TelegramUpdate};

/// Telegram API error
#[derive(Debug, Error)]
pub enum TelegramError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Telegram API error: {code} - {message}")]
    Api { code: i32, message: String },

    #[error("Invalid bot token")]
    InvalidToken,

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Configuration not found for role: {0}")]
    ConfigNotFound(String),
}

/// Telegram API response wrapper
#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
    error_code: Option<i32>,
}

/// Inbound telegram message for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundTelegramMessage {
    pub role_id: String,
    pub chat_id: i64,
    pub text: String,
    pub update_id: i64,
}

/// Telegram service for Bot API integration
#[derive(Clone)]
pub struct TelegramService {
    client: Client,
    /// Per-bot offset tracking for long polling: bot_token -> last_update_id
    offsets: Arc<RwLock<HashMap<String, i64>>>,
    /// Broadcast channel for routing inbound messages to handlers
    message_sender: broadcast::Sender<InboundTelegramMessage>,
    /// Active polling tasks: role_id -> shutdown oneshot sender (drop to signal stop)
    active_polls: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

impl TelegramService {
    /// Create new Telegram service
    pub fn new() -> Self {
        let (message_sender, _) = broadcast::channel(1000);
        Self {
            client: Client::new(),
            offsets: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            active_polls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the broadcast receiver for inbound messages
    pub fn subscribe(&self) -> broadcast::Receiver<InboundTelegramMessage> {
        self.message_sender.subscribe()
    }

    /// Start long polling for a specific bot in the background.
    /// If polling is already running for this role_id, does nothing.
    /// Returns immediately.
    pub fn start_polling(&self, role_id: String, bot_token: String) {
        // Check if already polling
        {
            let polls = self.active_polls.read();
            if polls.contains_key(&role_id) {
                tracing::info!("Polling already running for role {}", role_id);
                return;
            }
        }

        let offsets = Arc::clone(&self.offsets);
        let message_sender = self.message_sender.clone();
        let active_polls = Arc::clone(&self.active_polls);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

        // Register shutdown sender before starting
        {
            let mut polls = active_polls.write();
            polls.insert(role_id.clone(), shutdown_tx);
        }

        // Spawn background task
        tokio::spawn(async move {
            let client = Client::builder()
                .timeout(Duration::from_secs(310))
                .build()
                .unwrap_or_else(|_| Client::new());

            let mut offset = {
                let offsets_guard = offsets.read();
                offsets_guard.get(&bot_token).copied().unwrap_or(0)
            };

            tracing::info!("Starting Telegram long polling for role {}", role_id);

            // Convert oneshot to broadcast-like behavior by polling the receiver each iteration
            loop {
                tokio::select! {
                    // Create a new biased recv on each iteration by shadowing
                    _ = &mut shutdown_rx => {
                        tracing::info!("Long polling shutdown for role {}", role_id);
                        break;
                    }
                    result = Self::fetch_updates_inner(&client, &bot_token, offset) => {
                        match result {
                            Ok(new_updates) => {
                                for update in &new_updates {
                                    offset = update.update_id + 1;

                                    if let Some(text) = &update.text {
                                        if !text.is_empty() {
                                            let inbound = InboundTelegramMessage {
                                                role_id: role_id.clone(),
                                                chat_id: update.chat_id,
                                                text: text.clone(),
                                                update_id: update.update_id,
                                            };
                                            let _ = message_sender.send(inbound);
                                        }
                                    }
                                }

                                // Persist offset
                                {
                                    let mut offsets_guard = offsets.write();
                                    offsets_guard.insert(bot_token.clone(), offset);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error fetching Telegram updates: {}", e);
                                sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
            }

            // Cleanup
            {
                let mut polls = active_polls.write();
                polls.remove(&role_id);
            }
        });
    }

    /// Stop polling for a specific role
    pub fn stop_polling(&self, role_id: &str) {
        // Remove the sender - dropping it will cause the receiver to return Closed
        // The polling task will exit on its next iteration
        let mut polls = self.active_polls.write();
        if polls.remove(role_id).is_some() {
            tracing::info!("Stopping Telegram polling for role {}", role_id);
        }
    }

    /// Check if polling is active for a role
    pub fn is_polling(&self, role_id: &str) -> bool {
        let polls = self.active_polls.read();
        polls.contains_key(role_id)
    }

    /// Fetch updates helper (static to avoid self lifetime issues in spawned task)
    async fn fetch_updates_inner(
        client: &Client,
        bot_token: &str,
        offset: i64,
    ) -> Result<Vec<TelegramUpdate>, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", bot_token);
        let mut url_with_offset = format!("{}?timeout=30", url);
        if offset > 0 {
            url_with_offset.push_str(&format!("&offset={}", offset));
        }

        let response = client
            .get(&url_with_offset)
            .send()
            .await?
            .json::<TelegramResponse<Vec<TelegramUpdate>>>()
            .await?;

        if !response.ok {
            return Err(TelegramError::Api {
                code: response.error_code.unwrap_or(0),
                message: response.description.unwrap_or_default(),
            });
        }

        Ok(response.result.unwrap_or_default())
    }

    /// Send a message via a bot
    pub async fn send_message(
        &self,
        bot_token: &str,
        chat_id: &str,
        text: &str,
    ) -> Result<(), TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

        let params = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
        });

        let response = self.client.post(&url).json(&params).send().await?;

        let api_response: TelegramResponse<()> = response.json().await?;

        if !api_response.ok {
            let code = api_response.error_code.unwrap_or(0);
            let message = api_response.description.unwrap_or_else(|| "Unknown error".to_string());
            return Err(TelegramError::Api { code, message });
        }

        Ok(())
    }

    /// Broadcast a notification to all configured bots in a team
    pub async fn broadcast_notification(
        &self,
        configs: &[TelegramBotConfig],
        message: &str,
    ) -> Vec<Result<(), TelegramError>> {
        let mut results = Vec::new();

        for config in configs {
            if !config.enabled || !config.notifications_enabled {
                continue;
            }

            if let Some(chat_id) = &config.chat_id {
                let result = self
                    .send_message(&config.bot_token, chat_id, message)
                    .await;
                results.push(result);
            }
        }

        results
    }

    /// Verify a bot token is valid
    pub async fn verify_bot_token(&self, bot_token: &str) -> Result<bool, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/getMe", bot_token);

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct GetMeResponse {
            ok: bool,
            result: Option<BotInfo>,
        }

        #[derive(Deserialize)]
        struct BotInfo {
            id: i64,
            is_bot: bool,
            first_name: String,
            username: String,
        }

        let api_response: GetMeResponse = response.json().await?;

        Ok(api_response.ok && api_response.result.map(|b| b.is_bot).unwrap_or(false))
    }

    /// Get chat info for a specific chat ID
    pub async fn get_chat(
        &self,
        bot_token: &str,
        chat_id: &str,
    ) -> Result<ChatInfo, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/getChat", bot_token);

        let params = serde_json::json!({
            "chat_id": chat_id,
        });

        let response = self.client.post(&url).json(&params).send().await?;

        let api_response: TelegramResponse<ChatInfo> = response.json().await?;

        if !api_response.ok {
            let code = api_response.error_code.unwrap_or(0);
            let message = api_response.description.unwrap_or_else(|| "Unknown error".to_string());
            return Err(TelegramError::Api { code, message });
        }

        api_response
            .result
            .ok_or_else(|| TelegramError::SendFailed("No chat info returned".to_string()))
    }

    /// Set webhook for a bot (alternative to long polling)
    pub async fn set_webhook(
        &self,
        bot_token: &str,
        webhook_url: &str,
    ) -> Result<(), TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/setWebhook", bot_token);

        let params = serde_json::json!({
            "url": webhook_url,
        });

        let response = self.client.post(&url).json(&params).send().await?;

        let api_response: TelegramResponse<bool> = response.json().await?;

        if !api_response.ok {
            let code = api_response.error_code.unwrap_or(0);
            let message = api_response.description.unwrap_or_else(|| "Unknown error".to_string());
            return Err(TelegramError::Api { code, message });
        }

        Ok(())
    }
}

impl Default for TelegramService {
    fn default() -> Self {
        Self::new()
    }
}

/// Chat information from Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInfo {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub title: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_service_creation() {
        let service = TelegramService::new();
        let rx = service.subscribe();
        assert!(rx.len() == 0);
    }
}