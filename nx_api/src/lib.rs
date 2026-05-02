//! NexusFlow API 服务器
//!
//! 完整的 REST API 和 WebSocket 服务器。

#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments,
    non_camel_case_types,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::derivable_impls,
    clippy::should_implement_trait,
    clippy::needless_return,
    clippy::unnecessary_cast,
    clippy::useless_conversion,
    clippy::match_single_binding,
    clippy::let_and_return,
    clippy::needless_question_mark,
    clippy::unnecessary_lazy_evaluations,
    clippy::redundant_field_names,
    clippy::never_loop,
    clippy::useless_format,
    clippy::manual_strip,
    clippy::unnecessary_unwrap,
    clippy::unwrap_or_default,
    clippy::map_identity,
    clippy::enum_variant_names,
    clippy::new_without_default,
    clippy::borrowed_box,
    clippy::match_like_matches_macro,
    clippy::single_match,
    clippy::manual_unwrap_or_default,
    clippy::expect_fun_call,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::get_first,
    dropping_references,
    unreachable_patterns,
    clippy::if_same_then_else
)]

pub mod a2ui;
pub mod config;
pub mod error;
pub mod middleware;
pub mod migrations;
pub mod models;
pub mod response;
pub mod routes;
pub mod search;
pub mod services;
pub mod wisdom;
pub mod ws;

pub use a2ui::{A2UIService, A2UISession, A2UISessionManager};
pub use config::ApiConfig;
pub use routes::create_router;
pub use search::SearchMode;
pub use services::{ExecutionService, SessionService, WorkflowService};
pub use wisdom::WisdomService;
pub use ws::WebSocketHandler;
