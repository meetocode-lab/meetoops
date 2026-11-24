use crate::auth::{
    build_clear_session_cookie, build_session_cookie, client_ip, is_https_request, require_auth,
    require_permission,
};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, Method},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use meetoops_common::{ApiResponse, PaginationMeta, Role, SetupStage};
use meetoops_core::{check_port_availability, AppSettingsDto};
use meetoops_docker::probe_local_docker;
use meetoops_monitoring::collect_host_snapshot;
use meetoops_security::{permissions_for_role, Permission};
use serde::{Deserialize, Serialize};

pub fn api_router() -> Router<AppState> {
    Router::new()
        // Public health & setup status
        .route("/api/v1/health", get(health))
        .route("/api/v1/setup/status", get(setup_status))
        .route("/api/v1/setup/admin", post(setup_create_admin))
        .route("/api/v1/setup/preferences", post(setup_save_preferences))
        .route("/api/v1/setup/docker/test-local", post(setup_test_local_docker))
        .route(
            "/api/v1/setup/docker/connect-local",
            post(setup_connect_local_docker),
        )
        .route(
            "/api/v1/setup/docker/remote-token",
            post(setup_create_remote_token),
        )
        .route("/api/v1/setup/complete", post(setup_complete))
        // Auth
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/logout", post(auth_logout))
        .route("/api/v1/auth/me", get(auth_me))
        // Dashboard & Servers
        .route("/api/v1/dashboard/summary", get(dashboard_summary))
        .route("/api/v1/servers", get(list_servers))
        .route("/api/v1/servers/remote-token", post(create_remote_token))
        // Users
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/users/{id}", delete(delete_user))
        // Settings & Port Availability Check
        .route("/api/v1/settings", get(get_settings).patch(update_settings))
        .route(
            "/api/v1/settings/network/check-port",
            post(check_network_port),
        )
        // Audit Logs
        .route("/api/v1/audit-logs", get(list_audit_logs))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    setup_stage: SetupStage,
    active_port: u16,
    database: &'static str,
    vault_fingerprint: String,
}

async fn health(State(state): State<AppState>) -> Result<Json<ApiResponse<HealthResponse>>, ApiError> {
    let app_state = state.store.get_app_state().await?;
    Ok(Json(ApiResponse::ok(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        setup_stage: app_state.setup_stage,
        active_port: state.active_port,
        database: "sqlite_wal",
        vault_fingerprint: app_state.key_fingerprint_short,
    })))
}

#[derive(Serialize)]
struct SetupStatusResponse {
    setup_stage: SetupStage,
    initialized: bool,
    active_port: u16,
    language: String,
    app_name: String,
}

async fn setup_status(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SetupStatusResponse>>, ApiError> {
    let app_state = state.store.get_app_state().await?;
    let settings = state.store.get_settings().await.unwrap_or_default();
    Ok(Json(ApiResponse::ok(SetupStatusResponse {
        setup_stage: app_state.setup_stage,
        initialized: app_state.setup_stage.is_ready(),
        active_port: state.active_port,
        language: settings.language,
        app_name: settings.app_name,
    })))
}

#[derive(Deserialize)]
struct SetupAdminRequest {
    full_name: String,
    email: String,
    username: Option<String>,
    password: String,
}

#[derive(Serialize)]
struct AuthSessionResponse {
    user: meetoops_core::UserPublic,
    permissions: Vec<&'static str>,
    csrf_token: String,
    setup_stage: SetupStage,
}

async fn setup_create_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetupAdminRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = client_ip(&headers, state.config.trust_proxy);
    if !state.setup_rate_limiter.check(&ip) {
        return Err(ApiError::too_many_requests(
            "Too many setup requests. Please wait a minute.",
        ));
    }

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let (user, session_token, csrf_token) = state
        .store
        .create_initial_super_admin(
            &payload.full_name,
            &payload.email,
            payload.username.as_deref(),
            &payload.password,
            Some(&ip),
            user_agent,
        )
        .await?;

    let secure = is_https_request(&headers, state.config.trust_proxy);
    let cookie = build_session_cookie(&session_token, 24 * 3600, secure);
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(header::SET_COOKIE, cookie);

    let body = AuthSessionResponse {
        permissions: permissions_for_role(user.role),
        user,
        csrf_token,
        setup_stage: SetupStage::AdminCreated,
    };

    Ok((resp_headers, Json(ApiResponse::ok(body))))
}

#[derive(Deserialize)]
struct SetupPreferencesRequest {
    app_name: String,
    language: String,
    timezone: String,
    date_format: String,
    time_format: String,
    monitoring_interval_secs: u32,
}

async fn setup_save_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<SetupPreferencesRequest>,
) -> Result<Json<ApiResponse<AppSettingsDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::SettingsWrite).await?;
    let app_state = state.store.get_app_state().await?;
    if app_state.setup_stage.is_ready() {
        return Err(ApiError::from(meetoops_core::CoreError::SetupLocked));
    }

    let mut current = state.store.get_settings().await?;
    current.app_name = payload.app_name;
    current.language = payload.language;
    current.timezone = payload.timezone;
    current.date_format = payload.date_format;
    current.time_format = payload.time_format;
    current.monitoring_interval_secs = payload.monitoring_interval_secs;

    state
        .store
        .update_settings(&current, Some(&session.user.id))
        .await?;
    state
        .store
        .set_setup_stage(SetupStage::InfrastructureSetup)
        .await?;

    Ok(Json(ApiResponse::ok(current)))
}

async fn setup_test_local_docker(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<meetoops_docker::LocalDockerProbe>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::ServersWrite).await?;
    let probe = probe_local_docker().await;
    Ok(Json(ApiResponse::ok(probe)))
}

#[derive(Deserialize)]
struct ConnectLocalDockerRequest {
    name: Option<String>,
}

async fn setup_connect_local_docker(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<ConnectLocalDockerRequest>,
) -> Result<Json<ApiResponse<meetoops_core::ServerSummary>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersWrite).await?;
    let probe = probe_local_docker().await;
    let name = payload
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Local Docker".to_string());

    let server = state
        .store
        .register_local_server(
            &name,
            &probe.endpoint,
            probe.docker_version.as_deref(),
            probe.operating_system.as_deref(),
            probe.available,
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "server.connect_local",
            "server",
            Some(&server.id),
            Some(&server.id),
            "success",
            Some(&ip),
            serde_json::json!({ "available": probe.available, "endpoint": probe.endpoint }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(server)))
}

#[derive(Deserialize)]
struct RemoteTokenRequest {
    name: String,
    group_name: Option<String>,
}

#[derive(Serialize)]
struct RemoteTokenResponse {
    server: meetoops_core::ServerSummary,
    registration_token: String,
    expires_at: String,
    install_command: String,
}

async fn setup_create_remote_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<RemoteTokenRequest>,
) -> Result<Json<ApiResponse<RemoteTokenResponse>>, ApiError> {
    create_remote_token(State(state), headers, method, Json(payload)).await
}

async fn create_remote_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<RemoteTokenRequest>,
) -> Result<Json<ApiResponse<RemoteTokenResponse>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersWrite).await?;
    if payload.name.trim().len() < 2 {
        return Err(ApiError::bad_request(
            "Server name must be at least 2 characters.",
        ));
    }

    let (server, raw_token, expires_at) = state
        .store
        .create_remote_server_with_token(
            &payload.name,
            payload.group_name.as_deref().unwrap_or("Default"),
            &session.user.id,
        )
        .await?;

    let settings = state.store.get_settings().await.unwrap_or_default();
    let install_command = format!(
        "docker run -d --name meetoops-agent --restart unless-stopped -v /var/run/docker.sock:/var/run/docker.sock -e MEETOOPS_SERVER_URL={} -e MEETOOPS_REGISTRATION_TOKEN={} ghcr.io/meetoops/meetoops-agent:latest",
        settings.app_url, raw_token
    );

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "agent.registration_token_created",
            "server",
            Some(&server.id),
            Some(&server.id),
            "success",
            Some(&ip),
            serde_json::json!({ "server_name": server.name, "expires_at": expires_at }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(RemoteTokenResponse {
        server,
        registration_token: raw_token,
        expires_at,
        install_command,
    })))
}

async fn setup_complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::SettingsWrite).await?;
    state.store.set_setup_stage(SetupStage::Ready).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "setup.completed",
            "setting",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "stage": "READY" }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "setup_stage": SetupStage::Ready
    }))))
}

#[derive(Deserialize)]
struct LoginRequest {
    identifier: String,
    password: String,
}

async fn auth_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = client_ip(&headers, state.config.trust_proxy);
    if !state.auth_rate_limiter.check(&ip) {
        return Err(ApiError::too_many_requests(
            "Too many login attempts. Please wait a minute.",
        ));
    }

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let (user, session_token, csrf_token) = state
        .store
        .authenticate_user(&payload.identifier, &payload.password, Some(&ip), user_agent)
        .await?;

    let settings = state.store.get_settings().await.unwrap_or_default();
    let app_state = state.store.get_app_state().await?;
    let secure = is_https_request(&headers, state.config.trust_proxy);
    let cookie = build_session_cookie(
        &session_token,
        (settings.session_timeout_hours.max(1) as i64) * 3600,
        secure,
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(header::SET_COOKIE, cookie);

    let body = AuthSessionResponse {
        permissions: permissions_for_role(user.role),
        user,
        csrf_token,
        setup_stage: app_state.setup_stage,
    };

    Ok((resp_headers, Json(ApiResponse::ok(body))))
}

async fn auth_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<impl IntoResponse, ApiError> {
    let session = require_auth(&state, &headers, &method).await?;
    state.store.revoke_session(&session.session_id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "auth.logout",
            "session",
            Some(&session.session_id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    let secure = is_https_request(&headers, state.config.trust_proxy);
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(header::SET_COOKIE, build_clear_session_cookie(secure));
    Ok((resp_headers, Json(ApiResponse::ok(serde_json::json!({ "ok": true })))))
}

async fn auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<AuthSessionResponse>>, ApiError> {
    let session = require_auth(&state, &headers, &method).await?;
    let app_state = state.store.get_app_state().await?;
    Ok(Json(ApiResponse::ok(AuthSessionResponse {
        permissions: permissions_for_role(session.user.role),
        user: session.user,
        csrf_token: session.csrf_token,
        setup_stage: app_state.setup_stage,
    })))
}

#[derive(Serialize)]
struct DashboardSummaryResponse {
    servers_total: usize,
    servers_online: usize,
    servers_offline: usize,
    containers_total: Option<i64>,
    containers_running: Option<i64>,
    containers_stopped: Option<i64>,
    containers_unhealthy: Option<i64>,
    host_metrics: meetoops_monitoring::HostMetricsSnapshot,
    local_docker: meetoops_docker::LocalDockerProbe,
    recent_audit_events: Vec<meetoops_core::AuditLogEntry>,
    active_port: u16,
    configured_port: u16,
}

async fn dashboard_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<DashboardSummaryResponse>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DashboardRead).await?;

    let servers = state.store.list_servers().await?;
    let servers_total = servers.len();
    let servers_online = servers.iter().filter(|s| s.status == "online").count();
    let servers_offline = servers_total.saturating_sub(servers_online);

    let host_metrics = tokio::task::spawn_blocking(collect_host_snapshot)
        .await
        .map_err(|_| ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "METRICS_ERROR",
            "Failed to collect host telemetry.",
        ))?;

    let local_docker = probe_local_docker().await;
    let recent_audit_events = state.store.list_audit_logs(10).await?;
    let settings = state.store.get_settings().await.unwrap_or_default();

    Ok(Json(ApiResponse::ok(DashboardSummaryResponse {
        servers_total,
        servers_online,
        servers_offline,
        containers_total: local_docker.containers_total,
        containers_running: local_docker.containers_running,
        containers_stopped: local_docker.containers_stopped,
        containers_unhealthy: None, // Populated in Phase 1/2 once container health inspection streams are active
        host_metrics,
        local_docker,
        recent_audit_events,
        active_port: state.active_port,
        configured_port: settings.listen_port,
    })))
}

async fn list_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::ServerSummary>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::ServersRead).await?;
    let servers = state.store.list_servers().await?;
    Ok(Json(ApiResponse::ok(servers)))
}

#[derive(Deserialize)]
struct UsersQuery {
    search: Option<String>,
    role: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<UsersQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::UserPublic>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::UsersRead).await?;
    let roles: Vec<String> = q
        .role
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);
    let (users, total) = state
        .store
        .query_users_paginated(
            q.search.as_deref(),
            &roles,
            q.sort_by.as_deref().unwrap_or("created_at"),
            q.sort_dir.as_deref().unwrap_or("asc"),
            page,
            per_page,
        )
        .await?;
    Ok(Json(ApiResponse::paginated(
        users,
        PaginationMeta::new(page, per_page, total),
    )))
}

#[derive(Deserialize)]
struct CreateUserRequest {
    full_name: String,
    email: String,
    username: Option<String>,
    password: String,
    role: Role,
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<meetoops_core::UserPublic>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::UsersWrite).await?;
    if payload.role == Role::SuperAdmin && session.user.role != Role::SuperAdmin {
        return Err(ApiError::forbidden(
            "Only a Super Administrator can create another Super Administrator.",
        ));
    }

    let user = state
        .store
        .create_user(
            &payload.full_name,
            &payload.email,
            payload.username.as_deref(),
            &payload.password,
            payload.role,
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "user.create",
            "user",
            Some(&user.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "email": user.email, "role": user.role }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(user)))
}

async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::UsersDelete).await?;
    if session.user.id == id {
        return Err(ApiError::bad_request(
            "You cannot delete your own active account.",
        ));
    }

    state.store.delete_user(&id).await?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "user.delete",
            "user",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

#[derive(Serialize)]
struct SettingsResponse {
    settings: AppSettingsDto,
    active_port: u16,
    restart_required_for_port: bool,
    vault_fingerprint: String,
    env_port_override: Option<u16>,
}

async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<SettingsResponse>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::SettingsRead).await?;
    let settings = state.store.get_settings().await?;
    let app_state = state.store.get_app_state().await?;

    Ok(Json(ApiResponse::ok(SettingsResponse {
        restart_required_for_port: settings.listen_port != state.active_port,
        settings,
        active_port: state.active_port,
        vault_fingerprint: app_state.key_fingerprint_short,
        env_port_override: state.config.env_port_override,
    })))
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<AppSettingsDto>,
) -> Result<Json<ApiResponse<SettingsResponse>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::SettingsWrite).await?;

    // Verify port availability if port changed
    if payload.listen_port != state.active_port {
        let port_to_check = payload.listen_port;
        let active_port = state.active_port;
        let check = tokio::task::spawn_blocking(move || {
            check_port_availability(port_to_check, active_port)
        })
        .await
        .map_err(|_| ApiError::bad_request("Failed to probe port availability."))?;

        if !check.available {
            return Err(ApiError::bad_request(check.reason));
        }
    }

    state
        .store
        .update_settings(&payload, Some(&session.user.id))
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "settings.update",
            "setting",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({
                "app_name": payload.app_name,
                "language": payload.language,
                "listen_port": payload.listen_port,
                "monitoring_interval_secs": payload.monitoring_interval_secs
            }),
        )
        .await?;

    let app_state = state.store.get_app_state().await?;
    Ok(Json(ApiResponse::ok(SettingsResponse {
        restart_required_for_port: payload.listen_port != state.active_port,
        settings: payload,
        active_port: state.active_port,
        vault_fingerprint: app_state.key_fingerprint_short,
        env_port_override: state.config.env_port_override,
    })))
}

#[derive(Deserialize)]
struct CheckPortRequest {
    port: u16,
}

async fn check_network_port(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CheckPortRequest>,
) -> Result<Json<ApiResponse<meetoops_core::PortCheckResult>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::SettingsWrite).await?;
    let active_port = state.active_port;
    let result = tokio::task::spawn_blocking(move || {
        check_port_availability(payload.port, active_port)
    })
    .await
    .map_err(|_| ApiError::bad_request("Port availability probe failed."))?;

    Ok(Json(ApiResponse::ok(result)))
}

#[derive(Deserialize)]
struct AuditQuery {
    search: Option<String>,
    status: Option<String>,
    resource_type: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    limit: Option<u32>,
}

async fn list_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<AuditQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::AuditLogEntry>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::AuditRead).await?;
    let statuses: Vec<String> = q
        .status
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let resources: Vec<String> = q
        .resource_type
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.or(q.limit).unwrap_or(25).clamp(1, 200);

    let (entries, total) = state
        .store
        .query_audit_logs_paginated(
            q.search.as_deref(),
            &statuses,
            &resources,
            q.from.as_deref(),
            q.to.as_deref(),
            q.sort_by.as_deref().unwrap_or("created_at"),
            q.sort_dir.as_deref().unwrap_or("desc"),
            page,
            per_page,
        )
        .await?;
    Ok(Json(ApiResponse::paginated(
        entries,
        PaginationMeta::new(page, per_page, total),
    )))
}
