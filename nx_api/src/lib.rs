//! NexusFlow API 服务器
//!
//! 完整的 REST API 和 WebSocket 服务器。

pub mod config;
pub mod routes;
pub mod scheduler;
pub mod services;
pub mod middleware;
pub mod ws;
pub mod a2ui;
pub mod error;
pub mod wisdom;
pub mod search;
pub mod models;

pub use config::ApiConfig;
pub use routes::create_router;
pub use scheduler::{SchedulerService, init_scheduler, get_scheduler, QueueStats};
pub use services::{WorkflowService, ExecutionService, SessionService};
pub use ws::WebSocketHandler;
pub use a2ui::{A2UIService, A2UISession, A2UISessionManager};
pub use wisdom::WisdomService;
pub use search::SearchMode;