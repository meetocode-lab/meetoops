use crate::auth::{client_ip, require_permission};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use meetoops_common::{ApiResponse, PaginationMeta};
use meetoops_database::{
    browse_rows, execute_sql_query, inspect_columns, list_databases, list_tables,
    test_connection as test_db_connection, DatabaseExplorerError, DbConnectionConfig,
};
use meetoops_discovery::{detect_service, ContainerMetadataInput};
use meetoops_docker::list_containers;
use meetoops_redis::{
    delete_key, execute_command, get_key_detail, get_overview, mutate_collection_key, rename_key,
    scan_keys, set_string_key, test_connection as test_redis_connection, RedisConnectionConfig,
    RedisExplorerError,
};
use meetoops_security::{has_permission, Permission};
use serde::{Deserialize, Serialize};

pub fn explorer_router() -> Router<AppState> {
    Router::new()
        // Smart Service Discovery & Encrypted Credentials
        .route("/api/v1/services", get(api_list_services).post(api_create_manual_service))
        .route("/api/v1/services/scan", post(api_scan_services))
        .route("/api/v1/services/{id}", delete(api_delete_service))
        .route(
            "/api/v1/services/{id}/credentials",
            put(api_save_service_credentials),
        )
        // Database Explorer (PostgreSQL, MySQL/MariaDB, MongoDB)
        .route("/api/v1/services/{id}/db/databases", get(api_db_list_databases))
        .route("/api/v1/services/{id}/db/tables", get(api_db_list_tables))
        .route(
            "/api/v1/services/{id}/db/tables/{table}/columns",
            get(api_db_table_columns),
        )
        .route(
            "/api/v1/services/{id}/db/tables/{table}/rows",
            get(api_db_browse_rows),
        )
        .route("/api/v1/services/{id}/db/query", post(api_db_execute_query))
        // Redis Explorer
        .route("/api/v1/services/{id}/redis/overview", get(api_redis_overview))
        .route("/api/v1/services/{id}/redis/keys", get(api_redis_scan_keys))
        .route(
            "/api/v1/services/{id}/redis/key",
            get(api_redis_get_key)
                .put(api_redis_set_key)
                .delete(api_redis_delete_key),
        )
        .route("/api/v1/services/{id}/redis/rename", post(api_redis_rename_key))
        .route("/api/v1/services/{id}/redis/mutate", post(api_redis_mutate_key))
        .route("/api/v1/services/{id}/redis/command", post(api_redis_command))
}

#[derive(Serialize, Deserialize)]
struct StoredServiceSecret {
    username: String,
    password: String,
    database: Option<String>,
    redis_db: Option<i64>,
}

fn map_db_err(err: DatabaseExplorerError) -> ApiError {
    match err {
        DatabaseExplorerError::Validation(m) | DatabaseExplorerError::UnsupportedEngine(m) => {
            ApiError::bad_request(m)
        }
        DatabaseExplorerError::ReadOnlyBlocked => ApiError::forbidden(
            "Read-only mode is enabled on this connection. Mutating queries are blocked.",
        ),
        DatabaseExplorerError::Sql(e) => {
            ApiError::new(StatusCode::BAD_GATEWAY, "DB_QUERY_ERROR", format!("Database error: {e}"))
        }
        DatabaseExplorerError::Mongo(e) => {
            ApiError::new(StatusCode::BAD_GATEWAY, "MONGO_ERROR", format!("MongoDB error: {e}"))
        }
    }
}

fn map_redis_err(err: RedisExplorerError) -> ApiError {
    match err {
        RedisExplorerError::Validation(m) => ApiError::bad_request(m),
        RedisExplorerError::DestructiveCommandBlocked(cmd) => ApiError::forbidden(format!(
            "Destructive Redis command '{cmd}' requires explicit confirmation and redis:admin permission."
        )),
        RedisExplorerError::Redis(e) => {
            ApiError::new(StatusCode::BAD_GATEWAY, "REDIS_ERROR", format!("Redis error: {e}"))
        }
    }
}

fn resolve_reachable_host(host: &str) -> String {
    let trimmed = host.trim();
    if std::path::Path::new("/.dockerenv").exists()
        && matches!(trimmed, "127.0.0.1" | "localhost" | "::1")
        && std::net::ToSocketAddrs::to_socket_addrs(&("host.docker.internal", 80)).is_ok()
    {
        "host.docker.internal".to_string()
    } else {
        trimmed.to_string()
    }
}

async fn load_db_config(
    state: &AppState,
    service_id: &str,
) -> Result<(meetoops_core::DiscoveredServiceDto, DbConnectionConfig), ApiError> {
    let svc = state.store.get_service(service_id).await?;
    let secret_bytes = state
        .store
        .load_encrypted_secret(&state.vault, "service_connection", service_id)
        .await?
        .ok_or_else(|| {
            ApiError::bad_request("No credentials configured for this service yet.")
        })?;
    let secret: StoredServiceSecret = serde_json::from_slice(&secret_bytes)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "VAULT_DECODE", "Corrupted credential payload."))?;

    let host = svc
        .host
        .as_deref()
        .map(resolve_reachable_host)
        .ok_or_else(|| ApiError::bad_request("Service host is missing."))?;
    let port = svc
        .port
        .and_then(|p| u16::try_from(p).ok())
        .ok_or_else(|| ApiError::bad_request("Service port is missing."))?;

    let cfg = DbConnectionConfig {
        engine: svc.service_type.clone(),
        host,
        port,
        username: secret.username,
        password: secret.password,
        database: secret.database,
    };
    Ok((svc, cfg))
}

async fn load_redis_config(
    state: &AppState,
    service_id: &str,
) -> Result<(meetoops_core::DiscoveredServiceDto, RedisConnectionConfig), ApiError> {
    let svc = state.store.get_service(service_id).await?;
    let secret_bytes = state
        .store
        .load_encrypted_secret(&state.vault, "service_connection", service_id)
        .await?;
    let secret: StoredServiceSecret = secret_bytes
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(StoredServiceSecret {
            username: String::new(),
            password: String::new(),
            database: None,
            redis_db: Some(0),
        });

    let host = svc
        .host
        .as_deref()
        .map(resolve_reachable_host)
        .ok_or_else(|| ApiError::bad_request("Redis host is missing."))?;
    let port = svc
        .port
        .and_then(|p| u16::try_from(p).ok())
        .ok_or_else(|| ApiError::bad_request("Redis port is missing."))?;

    let cfg = RedisConnectionConfig {
        host,
        port,
        username: if secret.username.is_empty() {
            None
        } else {
            Some(secret.username)
        },
        password: if secret.password.is_empty() {
            None
        } else {
            Some(secret.password)
        },
        db: secret.redis_db,
    };
    Ok((svc, cfg))
}

#[derive(Deserialize)]
struct ListServicesQuery {
    search: Option<String>,
    service_type: Option<String>,
    readiness_stage: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn api_list_services(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<ListServicesQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::DiscoveredServiceDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DiscoveryRead).await?;
    let types: Vec<String> = q
        .service_type
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let stages: Vec<String> = q
        .readiness_stage
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);

    let (list, total) = state
        .store
        .query_services_paginated(
            q.search.as_deref(),
            &types,
            &stages,
            q.sort_by.as_deref().unwrap_or("created_at"),
            q.sort_dir.as_deref().unwrap_or("desc"),
            page,
            per_page,
        )
        .await?;
    Ok(Json(ApiResponse::paginated(
        list,
        PaginationMeta::new(page, per_page, total),
    )))
}

async fn api_scan_services(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::DiscoveredServiceDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DiscoveryRead).await?;
    let containers = list_containers(true).await.map_err(|e| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "DOCKER_UNAVAILABLE",
            format!("Cannot scan local containers: {e}"),
        )
    })?;

    for c in containers {
        let meta = ContainerMetadataInput {
            container_id: c.id.clone(),
            name: c.name.clone(),
            image: c.image.clone(),
            exposed_ports: c
                .ports
                .iter()
                .map(|p| (p.private_port, p.public_port))
                .collect(),
            labels: c.labels.clone(),
        };
        if let Some(cand) = detect_service(&meta) {
            state
                .store
                .upsert_discovered_service(
                    "local",
                    &cand.container_id,
                    &cand.name,
                    &cand.service_type,
                    cand.confidence.as_str(),
                    cand.readiness_stage.as_str(),
                    cand.host.as_deref(),
                    cand.port,
                    serde_json::json!({ "image": c.image, "signals": cand.signals }),
                )
                .await?;
        }
    }

    let list = state.store.list_services().await?;
    Ok(Json(ApiResponse::ok(list)))
}

#[derive(Deserialize)]
struct CreateManualServiceRequest {
    name: String,
    service_type: String,
    host: String,
    port: u16,
    read_only_mode: Option<bool>,
}

async fn api_create_manual_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateManualServiceRequest>,
) -> Result<Json<ApiResponse<meetoops_core::DiscoveredServiceDto>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::ServicesCredentials).await?;
    if !matches!(
        payload.service_type.as_str(),
        "postgres" | "mysql" | "mariadb" | "mongodb" | "redis"
    ) {
        return Err(ApiError::bad_request(
            "Supported explorer service types: postgres, mysql, mariadb, mongodb, redis.",
        ));
    }

    let svc = state
        .store
        .create_manual_service(
            &payload.name,
            &payload.service_type,
            &payload.host,
            payload.port,
            payload.read_only_mode.unwrap_or(true),
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "service.create_manual",
            "database",
            Some(&svc.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": svc.name, "service_type": svc.service_type }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(svc)))
}

#[derive(Deserialize)]
struct SaveCredentialsRequest {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    database: Option<String>,
    redis_db: Option<i64>,
    read_only_mode: Option<bool>,
}

#[derive(Serialize)]
struct VerifyCredentialsResponse {
    service: meetoops_core::DiscoveredServiceDto,
    auth_verified: bool,
    server_banner: Option<String>,
    verify_error: Option<String>,
}

async fn api_save_service_credentials(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<SaveCredentialsRequest>,
) -> Result<Json<ApiResponse<VerifyCredentialsResponse>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::ServicesCredentials).await?;
    let svc = state.store.get_service(&id).await?;

    let secret = StoredServiceSecret {
        username: payload.username.unwrap_or_default(),
        password: payload.password.unwrap_or_default(),
        database: payload.database.clone(),
        redis_db: payload.redis_db,
    };
    let plaintext = serde_json::to_vec(&secret).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ENCODE_ERROR",
            "Failed to encode credentials.",
        )
    })?;
    let secret_id = state
        .store
        .save_encrypted_secret(&state.vault, "service_connection", &id, &plaintext)
        .await?;

    // Live driver authentication test (rewriting loopback to host.docker.internal inside Docker)
    let effective_host = resolve_reachable_host(&payload.host);
    let (auth_verified, server_banner, verify_error) = match svc.service_type.as_str() {
        "postgres" | "mysql" | "mariadb" | "mongodb" => {
            let cfg = DbConnectionConfig {
                engine: svc.service_type.clone(),
                host: effective_host.clone(),
                port: payload.port,
                username: secret.username,
                password: secret.password,
                database: secret.database,
            };
            match test_db_connection(&cfg).await {
                Ok(banner) => (true, Some(banner), None),
                Err(e) => (false, None, Some(e.to_string())),
            }
        }
        "redis" => {
            let cfg = RedisConnectionConfig {
                host: effective_host.clone(),
                port: payload.port,
                username: if secret.username.is_empty() {
                    None
                } else {
                    Some(secret.username)
                },
                password: if secret.password.is_empty() {
                    None
                } else {
                    Some(secret.password)
                },
                db: secret.redis_db,
            };
            match test_redis_connection(&cfg).await {
                Ok(pong) => (true, Some(pong), None),
                Err(e) => (false, None, Some(e.to_string())),
            }
        }
        _ => (false, None, Some("No native explorer for this service type.".to_string())),
    };

    let next_stage = if auth_verified {
        "explorer_ready"
    } else {
        "credentials_available"
    };

    let updated = state
        .store
        .update_service_connection_and_readiness(
            &id,
            &payload.host,
            payload.port,
            &secret_id,
            payload.read_only_mode.unwrap_or(svc.read_only_mode),
            next_stage,
            auth_verified,
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "service.credentials_updated",
            "database",
            Some(&id),
            None,
            if auth_verified { "success" } else { "failure" },
            Some(&ip),
            serde_json::json!({ "service_type": svc.service_type, "auth_verified": auth_verified }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(VerifyCredentialsResponse {
        service: updated,
        auth_verified,
        server_banner,
        verify_error,
    })))
}

async fn api_delete_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::ServicesCredentials).await?;
    state.store.delete_service(&id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "service.delete",
            "database",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

// --- Database Explorer Handlers ---

async fn api_db_list_databases(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DbRead).await?;
    let (_svc, cfg) = load_db_config(&state, &id).await?;
    let dbs = list_databases(&cfg).await.map_err(map_db_err)?;
    Ok(Json(ApiResponse::ok(dbs)))
}

#[derive(Deserialize)]
struct DbQueryParam {
    database: String,
}

async fn api_db_list_tables(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<DbQueryParam>,
) -> Result<Json<ApiResponse<Vec<String>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DbRead).await?;
    let (_svc, cfg) = load_db_config(&state, &id).await?;
    let tables = list_tables(&cfg, &q.database).await.map_err(map_db_err)?;
    Ok(Json(ApiResponse::ok(tables)))
}

async fn api_db_table_columns(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((id, table)): Path<(String, String)>,
    Query(q): Query<DbQueryParam>,
) -> Result<Json<ApiResponse<Vec<meetoops_database::ColumnInfoDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DbRead).await?;
    let (_svc, cfg) = load_db_config(&state, &id).await?;
    let cols = inspect_columns(&cfg, &q.database, &table)
        .await
        .map_err(map_db_err)?;
    Ok(Json(ApiResponse::ok(cols)))
}

#[derive(Deserialize)]
struct BrowseRowsQuery {
    database: String,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn api_db_browse_rows(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((id, table)): Path<(String, String)>,
    Query(q): Query<BrowseRowsQuery>,
) -> Result<Json<ApiResponse<meetoops_database::QueryResultDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DbRead).await?;
    let (_svc, cfg) = load_db_config(&state, &id).await?;
    let res = browse_rows(
        &cfg,
        &q.database,
        &table,
        q.limit.unwrap_or(50),
        q.offset.unwrap_or(0),
    )
    .await
    .map_err(map_db_err)?;
    Ok(Json(ApiResponse::ok(res)))
}

#[derive(Deserialize)]
struct ExecuteSqlRequest {
    database: String,
    sql: String,
}

async fn api_db_execute_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<ExecuteSqlRequest>,
) -> Result<Json<ApiResponse<meetoops_database::QueryResultDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DbRead).await?;
    let (svc, cfg) = load_db_config(&state, &id).await?;

    let is_write = meetoops_database::is_mutating_sql(&payload.sql);
    if is_write && !has_permission(session.user.role, Permission::DbWrite) {
        return Err(ApiError::forbidden("Mutating SQL requires db:write permission."));
    }

    let res = execute_sql_query(&cfg, &payload.database, &payload.sql, svc.read_only_mode)
        .await
        .map_err(map_db_err)?;

    if is_write {
        let ip = client_ip(&headers, state.config.trust_proxy);
        state
            .store
            .write_audit_log(
                Some(&session.user.id),
                Some(&session.user.email),
                "db.query_write",
                "database",
                Some(&id),
                None,
                "success",
                Some(&ip),
                serde_json::json!({ "database": payload.database }),
            )
            .await?;
    }

    Ok(Json(ApiResponse::ok(res)))
}

// --- Redis Explorer Handlers ---

async fn api_redis_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<meetoops_redis::RedisOverviewDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::RedisRead).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    let overview = get_overview(&cfg).await.map_err(map_redis_err)?;
    Ok(Json(ApiResponse::ok(overview)))
}

#[derive(Deserialize)]
struct ScanKeysQuery {
    cursor: Option<u64>,
    pattern: Option<String>,
    count: Option<u32>,
}

async fn api_redis_scan_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<ScanKeysQuery>,
) -> Result<Json<ApiResponse<meetoops_redis::RedisScanPageDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::RedisRead).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    let page = scan_keys(
        &cfg,
        q.cursor.unwrap_or(0),
        q.pattern.as_deref().unwrap_or("*"),
        q.count.unwrap_or(50),
    )
    .await
    .map_err(map_redis_err)?;
    Ok(Json(ApiResponse::ok(page)))
}

#[derive(Deserialize)]
struct RedisKeyQuery {
    key: String,
}

async fn api_redis_get_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<RedisKeyQuery>,
) -> Result<Json<ApiResponse<meetoops_redis::RedisKeyDetailDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::RedisRead).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    let detail = get_key_detail(&cfg, &q.key).await.map_err(map_redis_err)?;
    Ok(Json(ApiResponse::ok(detail)))
}

#[derive(Deserialize)]
struct SetRedisKeyRequest {
    key: String,
    value: String,
    ttl_secs: Option<u64>,
}

async fn api_redis_set_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<SetRedisKeyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::RedisWrite).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    set_string_key(&cfg, &payload.key, &payload.value, payload.ttl_secs)
        .await
        .map_err(map_redis_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "redis.set_key",
            "redis",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "key": payload.key }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "saved": true }))))
}

async fn api_redis_delete_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<RedisKeyQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::RedisWrite).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    let deleted = delete_key(&cfg, &q.key).await.map_err(map_redis_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "redis.delete_key",
            "redis",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "key": q.key, "deleted": deleted }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": deleted }))))
}

#[derive(Deserialize)]
struct RenameRedisKeyRequest {
    old_key: String,
    new_key: String,
}

async fn api_redis_rename_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<RenameRedisKeyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::RedisWrite).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    rename_key(&cfg, &payload.old_key, &payload.new_key)
        .await
        .map_err(map_redis_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "redis.rename_key",
            "redis",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "old_key": payload.old_key, "new_key": payload.new_key }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "renamed": true }))))
}

#[derive(Deserialize)]
struct MutateRedisKeyRequest {
    key: String,
    key_type: String,
    field: Option<String>,
    value: String,
    score: Option<f64>,
}

async fn api_redis_mutate_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<MutateRedisKeyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::RedisWrite).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;
    mutate_collection_key(
        &cfg,
        &payload.key,
        &payload.key_type,
        payload.field.as_deref(),
        &payload.value,
        payload.score,
    )
    .await
    .map_err(map_redis_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "redis.mutate_key",
            "redis",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "key": payload.key, "key_type": payload.key_type }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "updated": true }))))
}

#[derive(Deserialize)]
struct RedisCommandRequest {
    command: String,
    confirm_destructive: Option<bool>,
}

async fn api_redis_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<RedisCommandRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::RedisWrite).await?;
    let (_svc, cfg) = load_redis_config(&state, &id).await?;

    let allow_destructive = payload.confirm_destructive.unwrap_or(false)
        && has_permission(session.user.role, Permission::RedisAdmin);

    let output = execute_command(&cfg, &payload.command, allow_destructive)
        .await
        .map_err(map_redis_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "redis.console_exec",
            "redis",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "cmd": payload.command.split_whitespace().next().unwrap_or("") }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "output": output }))))
}
