//! 插件路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::PluginInfo;

/// 列出所有已加载的插件
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PluginInfo>> {
    Json(state.plugin_service.list_plugins())
}

/// 获取插件信息
pub async fn get_plugin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PluginInfo>, (StatusCode, String)> {
    match state.plugin_service.get_plugin(&id) {
        Some(info) => Ok(Json(info)),
        None => Err((StatusCode::NOT_FOUND, format!("插件 {} 未找到", id))),
    }
}

/// 获取插件注册表状态
pub async fn get_plugin_registry_status(
    State(state): State<Arc<AppState>>,
) -> Json<PluginRegistryStatus> {
    Json(PluginRegistryStatus {
        loaded_count: state.plugin_service.count(),
        plugin_ids: state.plugin_service.list_plugins().into_iter().map(|p| p.id).collect(),
    })
}

/// 插件注册表状态
#[derive(Debug, serde::Serialize)]
pub struct PluginRegistryStatus {
    pub loaded_count: usize,
    pub plugin_ids: Vec<String>,
}