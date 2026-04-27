//! NexusFlow API 服务器
//!
//! 完整的 REST API 和 WebSocket 服务器。

pub mod a2ui;
pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod scheduler;
pub mod search;
pub mod services;
pub mod wisdom;
pub mod ws;

pub use a2ui::{A2UIService, A2UISession, A2UISessionManager};
pub use config::ApiConfig;
pub use routes::create_router;
pub use scheduler::{get_scheduler, init_scheduler, QueueStats, SchedulerService};
pub use search::SearchMode;
pub use services::{ExecutionService, SessionService, WorkflowService};
pub use wisdom::WisdomService;
pub use ws::WebSocketHandler;
