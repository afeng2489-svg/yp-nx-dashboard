use crate::models::team::TelegramBotConfig;
use crate::services::telegram_service::TelegramService;
use std::sync::Arc;

pub struct AlertService {
    telegram: Arc<TelegramService>,
    bot_configs: Arc<tokio::sync::RwLock<Vec<TelegramBotConfig>>>,
}

impl AlertService {
    pub fn new(telegram: Arc<TelegramService>) -> Self {
        Self {
            telegram,
            bot_configs: Arc::new(tokio::sync::RwLock::new(vec![])),
        }
    }

    pub async fn set_bot_configs(&self, configs: Vec<TelegramBotConfig>) {
        *self.bot_configs.write().await = configs;
    }

    pub async fn notify_stage_failure(
        &self,
        execution_id: &str,
        stage_name: &str,
        error: &str,
        retries: usize,
        rolled_back: bool,
    ) {
        let msg = format!(
            "NexusFlow 告警\n执行: {}\nStage: {} 失败\n错误: {}\n已重试: {} 次{}",
            execution_id,
            stage_name,
            &error[..error.len().min(200)],
            retries,
            if rolled_back { "\n已自动回滚" } else { "" }
        );
        self.send(&msg).await;
    }

    pub async fn notify_pipeline_failure(&self, execution_id: &str, error: &str) {
        let msg = format!(
            "NexusFlow 告警\nPipeline {} 整体失败\n错误: {}",
            execution_id,
            &error[..error.len().min(200)]
        );
        self.send(&msg).await;
    }

    async fn send(&self, msg: &str) {
        let configs = self.bot_configs.read().await.clone();
        if configs.is_empty() {
            return;
        }
        let telegram = self.telegram.clone();
        let msg = msg.to_string();
        tokio::spawn(async move {
            let _ = telegram.broadcast_notification(&configs, &msg).await;
        });
    }
}
