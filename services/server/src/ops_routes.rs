use crate::auth::{client_ip, require_permission};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use meetoops_common::{ApiResponse, PaginationMeta};
use meetoops_docker::{list_compose_stacks, run_container_exec, stack_action, ExecInputMsg};
use meetoops_security::Permission;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn ops_router() -> Router<AppState> {
    Router::new()
        // Interactive Container Terminal (xterm.js <-> Bollard PTY)
        .route("/ws/v1/docker/containers/{id}/exec", get(ws_container_exec))
        // Compose Stacks
        .route("/api/v1/docker/stacks", get(api_list_stacks))
        .route("/api/v1/docker/stacks/{project}/{action}", post(api_stack_action))
        // Multi-Server Management (Server-Side Paginated)
        .route("/api/v1/servers/query", get(api_query_servers))
        .route("/api/v1/servers/{id}", delete(api_delete_server))
        // Remote Agent Outbound Registration & Heartbeat
        .route("/api/v1/agent/register", post(api_agent_register))
        .route("/api/v1/agent/heartbeat", post(api_agent_heartbeat))
        // Alerts Engine
        .route(
            "/api/v1/alerts/rules",
            get(api_list_alert_rules).post(api_create_alert_rule),
        )
        .route(
            "/api/v1/alerts/rules/{id}",
            patch(api_toggle_alert_rule).delete(api_delete_alert_rule),
        )
        .route("/api/v1/alerts/events", get(api_query_alert_events))
        .route(
            "/api/v1/alerts/events/{id}/acknowledge",
            post(api_ack_alert_event),
        )
        .route(
            "/api/v1/alerts/events/{id}/resolve",
            post(api_resolve_alert_event),
        )
        .route("/api/v1/alerts/summary", get(api_alerts_summary))
        .route(
            "/api/v1/alerts/channels",
            get(api_list_alert_channels).post(api_create_alert_channel),
        )
        .route(
            "/api/v1/alerts/channels/{id}",
            patch(api_toggle_alert_channel).delete(api_delete_alert_channel),
        )
        .route(
            "/api/v1/alerts/channels/{id}/test",
            post(api_test_alert_channel),
        )
}

#[derive(Deserialize)]
struct ExecQuery {
    shell: Option<String>,
}

async fn ws_container_exec(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<ExecQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::TerminalExec).await?;
    let shell = q.shell.unwrap_or_else(|| "/bin/sh".to_string());
    let ip = client_ip(&headers, state.config.trust_proxy);

    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "terminal.exec_open",
            "container",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "shell": shell }),
        )
        .await?;

    Ok(ws.on_upgrade(move |socket| handle_exec_ws(socket, id, shell)))
}

async fn handle_exec_ws(socket: WebSocket, container_id: String, shell: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (stdin_tx, stdin_rx) = mpsc::channel::<ExecInputMsg>(128);
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<Vec<u8>>(128);
    let cancel = CancellationToken::new();

    let cancel_exec = cancel.clone();
    let exec_task = tokio::spawn(async move {
        if let Err(err) = run_container_exec(container_id, shell, stdin_rx, stdout_tx, cancel_exec).await {
            tracing::warn!(error = %err, "container exec session ended with error");
        }
    });

    let cancel_in = cancel.clone();
    let in_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(txt) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                        if v.get("type").and_then(|t| t.as_str()) == Some("resize") {
                            let cols = v.get("cols").and_then(|c| c.as_u64()).unwrap_or(80) as u16;
                            let rows = v.get("rows").and_then(|r| r.as_u64()).unwrap_or(24) as u16;
                            let _ = stdin_tx.send(ExecInputMsg::Resize { cols, rows }).await;
                            continue;
                        }
                        if v.get("type").and_then(|t| t.as_str()) == Some("input") {
                            if let Some(data) = v.get("data").and_then(|d| d.as_str()) {
                                if stdin_tx.send(ExecInputMsg::Data(data.as_bytes().to_vec())).await.is_err() {
                                    break;
                                }
                                continue;
                            }
                        }
                    }
                    if stdin_tx.send(ExecInputMsg::Data(txt.as_bytes().to_vec())).await.is_err() {
                        break;
                    }
                }
                Message::Binary(bin) => {
                    if stdin_tx.send(ExecInputMsg::Data(bin.to_vec())).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        cancel_in.cancel();
    });

    while let Some(bytes) = stdout_rx.recv().await {
        let text = String::from_utf8_lossy(&bytes).to_string();
        if ws_tx.send(Message::Text(text.into())).await.is_err() {
            break;
        }
    }

    cancel.cancel();
    in_task.abort();
    exec_task.abort();
}

async fn api_list_stacks(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::ComposeStackDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let stacks = list_compose_stacks().await.map_err(|e| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "DOCKER_UNAVAILABLE",
            format!("Unable to list Compose stacks: {e}"),
        )
    })?;
    Ok(Json(ApiResponse::ok(stacks)))
}

async fn api_stack_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((project, action)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let affected = stack_action(&project, &action).await.map_err(|e| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "STACK_ERROR",
            format!("Stack action failed: {e}"),
        )
    })?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            &format!("stack.{action}"),
            "container",
            Some(&project),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "affected_services": affected }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "project": project, "action": action, "affected": affected }),
    )))
}

#[derive(Deserialize)]
struct ServersQuery {
    search: Option<String>,
    status: Option<String>,
    connection_type: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn api_query_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<ServersQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::ServerSummary>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::ServersRead).await?;
    let statuses: Vec<String> = q
        .status
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let conn_types: Vec<String> = q
        .connection_type
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 200);

    let (items, total) = state
        .store
        .query_servers_paginated(
            q.search.as_deref(),
            &statuses,
            &conn_types,
            q.sort_by.as_deref().unwrap_or("created_at"),
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

async fn api_delete_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersDelete).await?;
    state.store.delete_server(&id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "server.delete",
            "server",
            Some(&id),
            Some(&id),
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

#[derive(Deserialize)]
struct AgentRegisterRequest {
    registration_token: String,
}

#[derive(Serialize)]
struct AgentRegisterResponse {
    server_id: String,
    agent_secret: String,
}

async fn api_agent_register(
    State(state): State<AppState>,
    Json(payload): Json<AgentRegisterRequest>,
) -> Result<Json<ApiResponse<AgentRegisterResponse>>, ApiError> {
    let (server_id, agent_secret) = state
        .store
        .register_agent_with_token(&payload.registration_token)
        .await?;
    Ok(Json(ApiResponse::ok(AgentRegisterResponse {
        server_id,
        agent_secret,
    })))
}

#[derive(Deserialize)]
struct AgentMetricPayload {
    cpu_percent: Option<f64>,
    mem_used_bytes: Option<i64>,
    mem_total_bytes: Option<i64>,
    mem_percent: Option<f64>,
    disk_used_bytes: Option<i64>,
    disk_total_bytes: Option<i64>,
    disk_percent: Option<f64>,
    disk_active_percent: Option<f64>,
    disk_read_bps: Option<f64>,
    disk_write_bps: Option<f64>,
    net_rx_bps: Option<f64>,
    net_tx_bps: Option<f64>,
    uptime_seconds: Option<i64>,
}

#[derive(Deserialize)]
struct AgentHeartbeatRequest {
    server_id: String,
    agent_secret: String,
    docker_version: Option<String>,
    os_info: Option<String>,
    cpu_cores: Option<i64>,
    memory_total_bytes: Option<i64>,
    agent_version: Option<String>,
    metrics: Option<AgentMetricPayload>,
}

async fn api_agent_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<AgentHeartbeatRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    state
        .store
        .record_agent_heartbeat(
            &payload.server_id,
            &payload.agent_secret,
            payload.docker_version.as_deref(),
            payload.os_info.as_deref(),
            payload.cpu_cores,
            payload.memory_total_bytes,
            payload.agent_version.as_deref(),
        )
        .await?;

    if let Some(m) = payload.metrics {
        let point = meetoops_core::HostMetricPoint {
            server_id: payload.server_id.clone(),
            cpu_percent: m.cpu_percent,
            mem_used_bytes: m.mem_used_bytes,
            mem_total_bytes: m.mem_total_bytes,
            mem_percent: m.mem_percent,
            disk_used_bytes: m.disk_used_bytes,
            disk_total_bytes: m.disk_total_bytes,
            disk_percent: m.disk_percent,
            disk_active_percent: m.disk_active_percent,
            disk_read_bps: m.disk_read_bps,
            disk_write_bps: m.disk_write_bps,
            net_rx_bps: m.net_rx_bps,
            net_tx_bps: m.net_tx_bps,
            uptime_seconds: m.uptime_seconds,
            recorded_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(err) = state.store.insert_host_metric(&point).await {
            tracing::warn!(server_id = %point.server_id, error = %err, "failed to persist remote agent metric");
        }
        if let Ok(fired) = state.store.evaluate_host_metric_alerts(&point).await {
            if !fired.is_empty() {
                let store_clone = state.store.clone();
                tokio::spawn(async move {
                    if let Ok(channels) = store_clone.list_alert_channels().await {
                        for ev in fired {
                            for ch in channels.iter().filter(|c| c.is_enabled) {
                                let _ = dispatch_webhook_notification(
                                    ch,
                                    &ev.rule_name,
                                    &ev.severity,
                                    &ev.message,
                                )
                                .await;
                            }
                        }
                    }
                });
            }
        }
        let _ = state.metrics_tx.send(point);
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({ "ok": true }))))
}

// --- Alerts Engine Handlers ---

async fn api_list_alert_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::AlertRuleDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsRead).await?;
    let rules = state.store.list_alert_rules().await?;
    Ok(Json(ApiResponse::ok(rules)))
}

#[derive(Deserialize)]
struct CreateAlertRuleRequest {
    name: String,
    metric_type: String,
    operator: String,
    threshold: f64,
    severity: String,
}

async fn api_create_alert_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateAlertRuleRequest>,
) -> Result<Json<ApiResponse<meetoops_core::AlertRuleDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    let rule = state
        .store
        .create_alert_rule(
            &payload.name,
            &payload.metric_type,
            &payload.operator,
            payload.threshold,
            &payload.severity,
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "alert.rule_create",
            "setting",
            Some(&rule.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": rule.name, "metric": rule.metric_type, "threshold": rule.threshold }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(rule)))
}

#[derive(Deserialize)]
struct ToggleAlertRuleRequest {
    is_enabled: bool,
}

async fn api_toggle_alert_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<ToggleAlertRuleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state
        .store
        .toggle_alert_rule(&id, payload.is_enabled)
        .await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "ok": true }))))
}

async fn api_delete_alert_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state.store.delete_alert_rule(&id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "alert.rule_delete",
            "setting",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

#[derive(Deserialize)]
struct AlertEventsQuery {
    search: Option<String>,
    severity: Option<String>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn api_query_alert_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<AlertEventsQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::AlertEventDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsRead).await?;
    let severities: Vec<String> = q
        .severity
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let statuses: Vec<String> = q
        .status
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 200);

    let (items, total) = state
        .store
        .query_alert_events_paginated(
            q.search.as_deref(),
            &severities,
            &statuses,
            q.from.as_deref(),
            q.to.as_deref(),
            q.sort_by.as_deref().unwrap_or("triggered_at"),
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

async fn api_ack_alert_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state
        .store
        .acknowledge_alert_event(&id, &session.user.email)
        .await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "acknowledged": true }))))
}

async fn api_resolve_alert_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state.store.resolve_alert_event(&id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "resolved": true }))))
}

async fn api_alerts_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsRead).await?;
    let (firing, acknowledged) = state.store.get_active_alerts_summary().await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "firing": firing,
        "acknowledged": acknowledged,
    }))))
}

async fn api_list_alert_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::AlertChannelDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsRead).await?;
    let channels = state.store.list_alert_channels().await?;
    Ok(Json(ApiResponse::ok(channels)))
}

#[derive(Deserialize)]
struct CreateAlertChannelRequest {
    name: String,
    channel_type: String,
    target_url: String,
}

async fn api_create_alert_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateAlertChannelRequest>,
) -> Result<Json<ApiResponse<meetoops_core::AlertChannelDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    let channel = state
        .store
        .create_alert_channel(&payload.name, &payload.channel_type, &payload.target_url)
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "alert.channel_create",
            "setting",
            Some(&channel.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": channel.name, "channel_type": channel.channel_type }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(channel)))
}

async fn api_toggle_alert_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<ToggleAlertRuleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state
        .store
        .toggle_alert_channel(&id, payload.is_enabled)
        .await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "ok": true }))))
}

async fn api_delete_alert_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    state.store.delete_alert_channel(&id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "alert.channel_delete",
            "setting",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

async fn api_test_alert_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AlertsWrite).await?;
    let channel = state.store.get_alert_channel(&id).await?;
    dispatch_webhook_notification(
        &channel,
        "MeetoOps Test Notification",
        "info",
        "This is a live test alert dispatched from MeetoOps Alert Center.",
    )
    .await
    .map_err(|e| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "WEBHOOK_DELIVERY_FAILED",
            format!("Webhook delivery failed: {e}"),
        )
    })?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "delivered": true }))))
}

pub async fn dispatch_webhook_notification(
    channel: &meetoops_core::AlertChannelDto,
    title: &str,
    severity: &str,
    message: &str,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let text = format!("[MeetoOps][{}] {}: {}", severity.to_uppercase(), title, message);
    let body = match channel.channel_type.as_str() {
        "slack" => serde_json::json!({ "text": text }),
        "discord" => serde_json::json!({ "content": text }),
        "telegram" => serde_json::json!({ "text": text }),
        _ => serde_json::json!({
            "source": "meetoops",
            "title": title,
            "severity": severity,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    };

    let res = client
        .post(&channel.target_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let code = res.status();
        let detail = res.text().await.unwrap_or_default();
        return Err(format!("HTTP {code}: {detail}"));
    }
    Ok(())
}
