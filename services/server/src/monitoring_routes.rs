use crate::auth::require_permission;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, HeaderMap, Method},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use meetoops_common::{ApiResponse, PaginationMeta};
use meetoops_core::HostMetricPoint;
use meetoops_monitoring::{
    collect_host_snapshot, render_prometheus_metrics, HostMetricsCollector,
};
use meetoops_security::Permission;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn monitoring_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/monitoring/host/history", get(host_metrics_history))
        .route("/api/v1/monitoring/host/logs", get(host_metrics_logs))
        .route("/ws/v1/monitoring/realtime", get(ws_realtime_metrics))
        .route("/metrics", get(prometheus_metrics))
}

/// Spawns the background host telemetry collector that samples real OS metrics,
/// persists them to SQLite, broadcasts live points to WebSocket clients, and prunes old rows.
pub fn spawn_metrics_collector(state: AppState) {
    let collector = Arc::new(Mutex::new(HostMetricsCollector::new()));
    tokio::spawn(async move {
        let mut tick_count: u64 = 0;
        loop {
            let interval_secs = state
                .store
                .get_settings()
                .await
                .map(|s| s.monitoring_interval_secs.clamp(2, 60) as u64)
                .unwrap_or(5);

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;

            let c = collector.clone();
            let snap_res = tokio::task::spawn_blocking(move || {
                let mut guard = c.lock().expect("metrics collector mutex");
                guard.sample()
            })
            .await;

            if let Ok(snap) = snap_res {
                let point = HostMetricPoint {
                    server_id: "local".to_string(),
                    cpu_percent: snap.cpu_usage_percent.map(|v| v as f64),
                    mem_used_bytes: snap.memory_used_bytes.map(|v| v as i64),
                    mem_total_bytes: snap.memory_total_bytes.map(|v| v as i64),
                    mem_percent: snap.memory_usage_percent.map(|v| v as f64),
                    disk_used_bytes: snap.disk_used_bytes.map(|v| v as i64),
                    disk_total_bytes: snap.disk_total_bytes.map(|v| v as i64),
                    disk_percent: snap.disk_usage_percent.map(|v| v as f64),
                    disk_active_percent: snap.disk_active_percent.map(|v| v as f64),
                    disk_read_bps: snap.disk_read_bps,
                    disk_write_bps: snap.disk_write_bps,
                    net_rx_bps: snap.net_rx_bps,
                    net_tx_bps: snap.net_tx_bps,
                    uptime_seconds: snap.uptime_seconds.map(|v| v as i64),
                    recorded_at: Utc::now().to_rfc3339(),
                };

                if let Err(err) = state.store.insert_host_metric(&point).await {
                    tracing::warn!(error = %err, "failed to persist host metric");
                }
                match state.store.evaluate_host_metric_alerts(&point).await {
                    Ok(fired) if !fired.is_empty() => {
                        let store_clone = state.store.clone();
                        tokio::spawn(async move {
                            if let Ok(channels) = store_clone.list_alert_channels().await {
                                for ev in fired {
                                    for ch in channels.iter().filter(|c| c.is_enabled) {
                                        if let Err(err) =
                                            crate::ops_routes::dispatch_webhook_notification(
                                                ch,
                                                &ev.rule_name,
                                                &ev.severity,
                                                &ev.message,
                                            )
                                            .await
                                        {
                                            tracing::warn!(
                                                channel = %ch.name,
                                                error = %err,
                                                "failed to dispatch alert webhook"
                                            );
                                        }
                                    }
                                }
                            }
                        });
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "failed to evaluate alert rules");
                    }
                    _ => {}
                }
                let _ = state.metrics_tx.send(point);
            }

            tick_count += 1;
            if tick_count % 360 == 0 {
                let _ = state.store.prune_old_metrics(7).await;
            }
        }
    });
}

#[derive(Deserialize)]
struct HistoryQuery {
    range: Option<String>,
    server_id: Option<String>,
}

async fn host_metrics_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<Vec<HostMetricPoint>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::MonitoringRead).await?;
    let minutes = match q.range.as_deref().unwrap_or("1h") {
        "15m" => 15,
        "1h" => 60,
        "6h" => 360,
        "24h" => 1440,
        "7d" => 10080,
        _ => 60,
    };
    let server_id = q.server_id.as_deref().unwrap_or("local");
    let points = state
        .store
        .query_host_metrics(server_id, minutes, 500)
        .await?;
    Ok(Json(ApiResponse::ok(points)))
}

#[derive(Deserialize)]
struct MetricsLogQuery {
    server_id: Option<String>,
    risk: Option<String>, // comma-separated: safe,warning,critical
    from: Option<String>,
    to: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn host_metrics_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<MetricsLogQuery>,
) -> Result<Json<ApiResponse<Vec<HostMetricPoint>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::MonitoringRead).await?;
    let server_id = q.server_id.as_deref().unwrap_or("local");
    let risks: Vec<String> = q
        .risk
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);

    let (items, total) = state
        .store
        .query_host_metrics_paginated(
            server_id,
            &risks,
            q.from.as_deref(),
            q.to.as_deref(),
            q.sort_by.as_deref().unwrap_or("recorded_at"),
            q.sort_dir.as_deref().unwrap_or("desc"),
            page,
            per_page,
        )
        .await?;

    Ok(Json(ApiResponse::paginated(
        items,
        PaginationMeta::new(page, per_page, total),
    )))
}

async fn ws_realtime_metrics(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<impl IntoResponse, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::MonitoringRead).await?;
    let rx = state.metrics_tx.subscribe();
    Ok(ws.on_upgrade(move |socket| handle_metrics_ws(socket, rx)))
}

async fn handle_metrics_ws(
    socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<HostMetricPoint>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let drain = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    loop {
        match rx.recv().await {
            Ok(point) => {
                if let Ok(json) = serde_json::to_string(&point) {
                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }

    drain.abort();
}

async fn prometheus_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<impl IntoResponse, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::MonitoringRead).await?;
    let snap = tokio::task::spawn_blocking(collect_host_snapshot)
        .await
        .map_err(|_| {
            ApiError::new(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "METRICS_ERROR",
                "Failed to sample host metrics.",
            )
        })?;
    let body = render_prometheus_metrics(&snap);
    Ok((
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    ))
}
