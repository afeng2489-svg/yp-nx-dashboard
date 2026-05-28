//! AF-04 工厂台本地指标：factory_events 表 + 聚合查询

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::routes::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/factory/events", post(record_event))
        .route("/api/v1/factory/metrics", get(get_metrics))
}

#[derive(Debug, Deserialize)]
pub struct RecordEventRequest {
    pub device_id: String,
    pub event_type: String,
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RecordEventResponse {
    pub ok: bool,
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct RateMetric {
    pub rate: f64,
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Serialize)]
pub struct MedianMetric {
    pub median_minutes: f64,
    pub samples: u64,
}

#[derive(Debug, Serialize)]
pub struct FactoryMetricsResponse {
    pub activation: RateMetric,
    pub golden_path_success: RateMetric,
    pub time_to_first_diff: MedianMetric,
    pub run_completion: RateMetric,
    pub terminal_fallback: RateMetric,
    pub w2_retention: RateMetric,
}

struct StoredEvent {
    device_id: String,
    event_type: String,
    execution_id: Option<String>,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

fn payload_bool(payload: &serde_json::Value, key: &str) -> bool {
    payload.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn payload_str(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload.get(key).and_then(|v| v.as_str()).map(String::from)
}

pub async fn record_event(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RecordEventRequest>,
) -> Json<RecordEventResponse> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let payload = serde_json::to_string(&req.payload).unwrap_or_else(|_| "{}".to_string());

    if let Ok(conn) = Connection::open(&state.db_path) {
        let _ = conn.execute(
            "INSERT INTO factory_events (id, device_id, event_type, execution_id, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, req.device_id, req.event_type, req.execution_id, payload, now],
        );
    }

    Json(RecordEventResponse { ok: true, id })
}

pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<FactoryMetricsResponse> {
    let events = load_events(&state.db_path);
    Json(compute_metrics(&events))
}

fn load_events(db_path: &str) -> Vec<StoredEvent> {
    let Ok(conn) = Connection::open(db_path) else {
        return Vec::new();
    };
    let mut stmt = match conn.prepare(
        "SELECT device_id, event_type, execution_id, payload, created_at
         FROM factory_events ORDER BY created_at ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let Ok(mut rows) = stmt.query([]) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let payload_str: String = row.get(3).unwrap_or_default();
        let payload: serde_json::Value =
            serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));
        let created_at: String = row.get(4).unwrap_or_default();
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        events.push(StoredEvent {
            device_id: row.get(0).unwrap_or_default(),
            event_type: row.get(1).unwrap_or_default(),
            execution_id: row.get(2).ok(),
            payload,
            created_at,
        });
    }
    events
}

fn rate(n: u64, d: u64) -> f64 {
    if d == 0 {
        0.0
    } else {
        (n as f64) / (d as f64)
    }
}

fn compute_metrics(events: &[StoredEvent]) -> FactoryMetricsResponse {
    use std::collections::{HashMap, HashSet};

    let mut opens_by_device: HashMap<String, Vec<DateTime<Utc>>> = HashMap::new();
    let mut first_open: HashMap<String, DateTime<Utc>> = HashMap::new();
    let mut activated_devices: HashSet<String> = HashSet::new();
    let mut opened_devices: HashSet<String> = HashSet::new();

    let mut gp_started = 0u64;
    let mut gp_success = 0u64;
    let mut run_started = 0u64;
    let mut run_completed = 0u64;
    let mut terminal_fallbacks = 0u64;
    let mut diff_minutes: Vec<f64> = Vec::new();

    for ev in events {
        match ev.event_type.as_str() {
            "factory_opened" => {
                opened_devices.insert(ev.device_id.clone());
                opens_by_device
                    .entry(ev.device_id.clone())
                    .or_default()
                    .push(ev.created_at);
                first_open
                    .entry(ev.device_id.clone())
                    .and_modify(|t| {
                        if ev.created_at < *t {
                            *t = ev.created_at;
                        }
                    })
                    .or_insert(ev.created_at);
            }
            "run_started" => {
                run_started += 1;
                if payload_bool(&ev.payload, "golden_path") {
                    gp_started += 1;
                }
                if let Some(first) = first_open.get(&ev.device_id) {
                    if ev.created_at <= *first + Duration::hours(24) {
                        activated_devices.insert(ev.device_id.clone());
                    }
                } else {
                    // 未记录 factory_opened 时仍计为激活
                    activated_devices.insert(ev.device_id.clone());
                    opened_devices.insert(ev.device_id.clone());
                }
            }
            "run_completed" => {
                let status = payload_str(&ev.payload, "status").unwrap_or_default();
                if status == "completed" {
                    run_completed += 1;
                }
                if payload_bool(&ev.payload, "golden_path") && status == "completed" {
                    gp_success += 1;
                }
            }
            "first_artifact" => {
                if let Some(ms) = ev.payload.get("elapsed_ms").and_then(|v| v.as_u64()) {
                    if payload_bool(&ev.payload, "golden_path") {
                        diff_minutes.push(ms as f64 / 60_000.0);
                    }
                }
            }
            "terminal_fallback" => {
                terminal_fallbacks += 1;
            }
            _ => {}
        }
    }

    let activation_d = opened_devices.len() as u64;
    let activation_n = activated_devices.len() as u64;

    let mut w2_d = 0u64;
    let mut w2_n = 0u64;
    let now = Utc::now();
    for (device, first) in &first_open {
        if now - *first < Duration::days(14) {
            continue;
        }
        w2_d += 1;
        let opens = opens_by_device.get(device).map(|v| v.as_slice()).unwrap_or(&[]);
        let returned = opens.iter().any(|t| *t >= *first + Duration::days(14));
        if returned {
            w2_n += 1;
        }
    }

    let median_minutes = if diff_minutes.is_empty() {
        0.0
    } else {
        diff_minutes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = diff_minutes.len() / 2;
        if diff_minutes.len() % 2 == 0 {
            (diff_minutes[mid - 1] + diff_minutes[mid]) / 2.0
        } else {
            diff_minutes[mid]
        }
    };

    FactoryMetricsResponse {
        activation: RateMetric {
            rate: rate(activation_n, activation_d),
            numerator: activation_n,
            denominator: activation_d,
        },
        golden_path_success: RateMetric {
            rate: rate(gp_success, gp_started),
            numerator: gp_success,
            denominator: gp_started,
        },
        time_to_first_diff: MedianMetric {
            median_minutes,
            samples: diff_minutes.len() as u64,
        },
        run_completion: RateMetric {
            rate: rate(run_completed, run_started),
            numerator: run_completed,
            denominator: run_started,
        },
        terminal_fallback: RateMetric {
            rate: rate(terminal_fallbacks, run_started),
            numerator: terminal_fallbacks,
            denominator: run_started,
        },
        w2_retention: RateMetric {
            rate: rate(w2_n, w2_d),
            numerator: w2_n,
            denominator: w2_d,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(
        device: &str,
        event_type: &str,
        payload: serde_json::Value,
        at: DateTime<Utc>,
    ) -> StoredEvent {
        StoredEvent {
            device_id: device.to_string(),
            event_type: event_type.to_string(),
            execution_id: None,
            payload,
            created_at: at,
        }
    }

    #[test]
    fn compute_activation_and_golden_path() {
        let t0 = Utc::now();
        let events = vec![
            ev("dev-1", "factory_opened", serde_json::json!({}), t0),
            ev(
                "dev-1",
                "run_started",
                serde_json::json!({"golden_path": true}),
                t0,
            ),
            ev(
                "dev-1",
                "run_completed",
                serde_json::json!({"golden_path": true, "status": "completed"}),
                t0,
            ),
        ];
        let m = compute_metrics(&events);
        assert_eq!(m.activation.denominator, 1);
        assert_eq!(m.activation.numerator, 1);
        assert_eq!(m.golden_path_success.numerator, 1);
        assert_eq!(m.golden_path_success.denominator, 1);
    }
}
