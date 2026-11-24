use crate::auth::{client_ip, require_auth, require_permission};
use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use meetoops_common::ApiResponse;
use meetoops_core::AppSettingsDto;
use meetoops_security::{hash_token, Permission};
use serde::{Deserialize, Serialize};

pub fn phase5_router() -> Router<AppState> {
    Router::new()
        // Personal API Tokens
        .route("/api/v1/auth/tokens", get(list_tokens).post(create_token))
        .route("/api/v1/auth/tokens/{id}", delete(revoke_token))
        // Backup & Restore (super_admin / backup:manage)
        .route("/api/v1/backup/export-settings", get(export_settings_backup))
        .route("/api/v1/backup/import-settings", post(import_settings_backup))
        .route(
            "/api/v1/backup/snapshots",
            get(list_db_snapshots).post(create_db_snapshot),
        )
        .route(
            "/api/v1/backup/snapshots/{filename}",
            get(download_db_snapshot),
        )
}

async fn list_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::ApiTokenDto>>>, ApiError> {
    let session = require_auth(&state, &headers, &method).await?;
    let tokens = state.store.list_user_api_tokens(&session.user.id).await?;
    Ok(Json(ApiResponse::ok(tokens)))
}

#[derive(Deserialize)]
struct CreateTokenRequest {
    name: String,
    expires_days: Option<i64>,
}

#[derive(Serialize)]
struct CreateTokenResponse {
    token: meetoops_core::ApiTokenDto,
    plaintext_token: String,
}

async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<Json<ApiResponse<CreateTokenResponse>>, ApiError> {
    let session = require_auth(&state, &headers, &method).await?;
    let (token, plaintext_token) = state
        .store
        .create_api_token(
            &session.user.id,
            &payload.name,
            vec!["role:inherit".to_string()],
            payload.expires_days,
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "auth.api_token_create",
            "user",
            Some(&token.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": token.name, "prefix": token.token_prefix }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(CreateTokenResponse {
        token,
        plaintext_token,
    })))
}

async fn revoke_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_auth(&state, &headers, &method).await?;
    state.store.revoke_api_token(&session.user.id, &id).await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "auth.api_token_revoke",
            "user",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "revoked": true }))))
}

#[derive(Serialize, Deserialize)]
struct SettingsBackupBundle {
    version: String,
    exported_at: String,
    settings: AppSettingsDto,
    sha256: String,
}

async fn export_settings_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<SettingsBackupBundle>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::BackupManage).await?;
    let settings = state.store.get_settings().await?;
    let canonical = serde_json::to_string(&settings).unwrap_or_default();
    let sha256 = hash_token(&canonical);

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "backup.export_settings",
            "setting",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "sha256_prefix": &sha256[..12] }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(SettingsBackupBundle {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: Utc::now().to_rfc3339(),
        settings,
        sha256,
    })))
}

async fn import_settings_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(bundle): Json<SettingsBackupBundle>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::BackupManage).await?;
    let canonical = serde_json::to_string(&bundle.settings).unwrap_or_default();
    let expected = hash_token(&canonical);
    if expected != bundle.sha256 {
        return Err(ApiError::bad_request(
            "Backup integrity check failed (SHA-256 mismatch).",
        ));
    }

    state
        .store
        .update_settings(&bundle.settings, Some(&session.user.id))
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "backup.import_settings",
            "setting",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "version": bundle.version }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "restored": true }))))
}

#[derive(Serialize)]
struct SnapshotInfoDto {
    filename: String,
    size_bytes: u64,
    created_at: String,
}

async fn list_db_snapshots(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<SnapshotInfoDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::BackupManage).await?;
    let dir = state.config.data_dir.join("backups");
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("db") {
                if let Ok(meta) = entry.metadata() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    let created_at = meta
                        .modified()
                        .ok()
                        .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339())
                        .unwrap_or_default();
                    items.push(SnapshotInfoDto {
                        filename: fname,
                        size_bytes: meta.len(),
                        created_at,
                    });
                }
            }
        }
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Json(ApiResponse::ok(items)))
}

async fn create_db_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<SnapshotInfoDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::BackupManage).await?;
    let dir = state.config.data_dir.join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "BACKUP_IO",
            format!("Failed to create backup directory: {e}"),
        )
    })?;

    let ts = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("meetoops-snapshot-{ts}.db");
    let target = dir.join(&filename);
    state.store.create_sqlite_snapshot(&target).await?;

    let size_bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "backup.create_db_snapshot",
            "database",
            Some(&filename),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "size_bytes": size_bytes }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(SnapshotInfoDto {
        filename,
        size_bytes,
        created_at: Utc::now().to_rfc3339(),
    })))
}

async fn download_db_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::BackupManage).await?;
    if filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || !filename.ends_with(".db")
    {
        return Err(ApiError::bad_request("Invalid snapshot filename."));
    }
    let path = state.config.data_dir.join("backups").join(&filename);
    let bytes = std::fs::read(&path).map_err(|_| {
        ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "Snapshot not found.")
    })?;

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    resp_headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{filename}\"")
            .parse()
            .unwrap(),
    );
    Ok((resp_headers, bytes))
}
