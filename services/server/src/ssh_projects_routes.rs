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
use meetoops_common::ApiResponse;
use meetoops_security::Permission;
use meetoops_ssh::{
    exec_command, provision_linux_sudo_user, run_interactive_pty, sanitize_linux_username,
    test_connection as test_ssh_connection, SshAuth, SshConnectionConfig, SshError, SshInputMsg,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn ssh_projects_router() -> Router<AppState> {
    Router::new()
        // Host SSH Terminal (Termius replacement)
        .route(
            "/api/v1/ssh/hosts",
            get(api_list_ssh_hosts).post(api_create_ssh_host),
        )
        .route("/api/v1/ssh/hosts/{id}", delete(api_delete_ssh_host))
        .route("/api/v1/ssh/hosts/{id}/test", post(api_test_ssh_host))
        .route(
            "/api/v1/ssh/hosts/{id}/provision-user",
            post(api_provision_ssh_user),
        )
        .route("/ws/v1/ssh/hosts/{id}/terminal", get(ws_ssh_terminal))
        // GitHub Projects & Live Compose Deploy
        .route(
            "/api/v1/projects",
            get(api_list_projects).post(api_create_project),
        )
        .route("/api/v1/projects/scan", post(api_scan_existing_projects))
        .route(
            "/api/v1/projects/{id}",
            patch(api_update_project).delete(api_delete_project),
        )
        .route("/api/v1/projects/{id}/env", get(api_reveal_project_env))
        .route(
            "/api/v1/projects/{id}/check-remote",
            post(api_check_project_remote),
        )
        .route("/api/v1/projects/{id}/deploy", post(api_deploy_project))
        .route(
            "/api/v1/projects/{id}/action/{action}",
            post(api_project_action),
        )
        .route(
            "/api/v1/projects/{id}/deployments",
            get(api_list_project_deployments),
        )
}

fn map_ssh_err(err: SshError) -> ApiError {
    match err {
        SshError::Validation(m) => ApiError::bad_request(m),
        SshError::Auth(m) => ApiError::new(StatusCode::UNAUTHORIZED, "SSH_AUTH_FAILED", m),
        SshError::Connect(m) => ApiError::new(StatusCode::BAD_GATEWAY, "SSH_CONNECT_FAILED", m),
        SshError::Protocol(m) => ApiError::new(StatusCode::BAD_GATEWAY, "SSH_ERROR", m),
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn inject_git_token(repo_url: &str, token: Option<&str>) -> String {
    let Some(tok) = token.map(str::trim).filter(|t| !t.is_empty()) else {
        return repo_url.to_string();
    };
    if let Some(rest) = repo_url.strip_prefix("https://") {
        if !rest.contains('@') {
            return format!("https://x-access-token:{tok}@{rest}");
        }
    }
    repo_url.to_string()
}

fn redact_token(text: &str, token: Option<&str>) -> String {
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(tok) => text.replace(tok, "[REDACTED]"),
        None => text.to_string(),
    }
}

async fn load_ssh_config(
    state: &AppState,
    host_id: &str,
) -> Result<(meetoops_core::SshHostDto, SshConnectionConfig), ApiError> {
    let host = state.store.get_ssh_host(host_id).await?;
    let secret_bytes = state
        .store
        .load_encrypted_secret(&state.vault, "ssh_host", host_id)
        .await?
        .ok_or_else(|| ApiError::bad_request("No SSH credentials stored for this host."))?;
    let auth: SshAuth = serde_json::from_slice(&secret_bytes).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VAULT_DECODE",
            "Corrupted SSH credentials.",
        )
    })?;
    let cfg = SshConnectionConfig {
        host: host.host.clone(),
        port: host.port.clamp(1, 65535) as u16,
        username: host.username.clone(),
        auth,
    };
    Ok((host, cfg))
}

// --- SSH Hosts Handlers ---

async fn api_list_ssh_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::SshHostDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::ServersRead).await?;
    let hosts = state.store.list_ssh_hosts().await?;
    Ok(Json(ApiResponse::ok(hosts)))
}

#[derive(Deserialize)]
struct CreateSshHostRequest {
    name: String,
    host: String,
    port: Option<u16>,
    username: String,
    auth_type: String,
    password: Option<String>,
    private_key_pem: Option<String>,
    passphrase: Option<String>,
    use_sudo: Option<bool>,
}

#[derive(Serialize)]
struct CreateSshHostResponse {
    host: meetoops_core::SshHostDto,
    verified: bool,
    banner: Option<String>,
    verify_error: Option<String>,
}

async fn api_create_ssh_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateSshHostRequest>,
) -> Result<Json<ApiResponse<CreateSshHostResponse>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersWrite).await?;
    let port = payload.port.unwrap_or(22);

    let auth = match payload.auth_type.as_str() {
        "password" => {
            let pw = payload.password.unwrap_or_default();
            if pw.is_empty() {
                return Err(ApiError::bad_request("SSH password is required."));
            }
            SshAuth::Password { password: pw }
        }
        "private_key" => {
            let pem = payload.private_key_pem.unwrap_or_default();
            if pem.trim().is_empty() {
                return Err(ApiError::bad_request("SSH private key PEM is required."));
            }
            SshAuth::PrivateKey {
                private_key_pem: pem,
                passphrase: payload.passphrase.filter(|p| !p.is_empty()),
            }
        }
        _ => {
            return Err(ApiError::bad_request(
                "auth_type must be 'password' or 'private_key'.",
            ));
        }
    };

    let host = state
        .store
        .create_ssh_host(
            &payload.name,
            &payload.host,
            port,
            &payload.username,
            &payload.auth_type,
            payload.use_sudo.unwrap_or(true),
        )
        .await?;

    let secret_bytes = serde_json::to_vec(&auth).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ENCODE_ERROR",
            "Failed to encode SSH secret.",
        )
    })?;
    state
        .store
        .save_encrypted_secret(&state.vault, "ssh_host", &host.id, &secret_bytes)
        .await?;

    let cfg = SshConnectionConfig {
        host: host.host.clone(),
        port,
        username: host.username.clone(),
        auth,
    };

    let (verified, banner, verify_error) = match test_ssh_connection(&cfg).await {
        Ok(b) => {
            let _ = state.store.update_ssh_host_banner(&host.id, &b).await;
            (true, Some(b), None)
        }
        Err(e) => (false, None, Some(e.to_string())),
    };

    let updated = state.store.get_ssh_host(&host.id).await?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "ssh.host_create",
            "server",
            Some(&updated.id),
            None,
            if verified { "success" } else { "failure" },
            Some(&ip),
            serde_json::json!({ "name": updated.name, "host": updated.host, "verified": verified }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(CreateSshHostResponse {
        host: updated,
        verified,
        banner,
        verify_error,
    })))
}

async fn api_test_ssh_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::TerminalExec).await?;
    let (_host, cfg) = load_ssh_config(&state, &id).await?;
    let banner = test_ssh_connection(&cfg).await.map_err(map_ssh_err)?;
    state.store.update_ssh_host_banner(&id, &banner).await?;
    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "connected": true, "banner": banner }),
    )))
}

#[derive(Deserialize)]
struct ProvisionUserRequest {
    linux_username: Option<String>,
    password: String,
    add_docker_group: Option<bool>,
}

async fn api_provision_ssh_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<ProvisionUserRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersWrite).await?;
    let (_host, admin_cfg) = load_ssh_config(&state, &id).await?;

    // Default to matching the logged-in MeetoOps user's username or email prefix
    let raw_name = payload
        .linux_username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(session.user.username.as_deref())
        .unwrap_or(&session.user.email);
    let target_user = sanitize_linux_username(raw_name);

    let id_output = provision_linux_sudo_user(
        &admin_cfg,
        &target_user,
        &payload.password,
        payload.add_docker_group.unwrap_or(true),
    )
    .await
    .map_err(map_ssh_err)?;

    // Switch this SSH host profile to authenticate as the newly provisioned sudo user
    let new_auth = SshAuth::Password {
        password: payload.password,
    };
    let secret_bytes = serde_json::to_vec(&new_auth).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ENCODE_ERROR",
            "Failed to encode user credentials.",
        )
    })?;
    state
        .store
        .save_encrypted_secret(&state.vault, "ssh_host", &id, &secret_bytes)
        .await?;
    let updated = state
        .store
        .update_ssh_host_provisioned_user(&id, &target_user)
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "ssh.provision_sudo_user",
            "server",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "linux_username": target_user, "id_output": id_output }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "host": updated,
        "linux_username": target_user,
        "id_output": id_output,
    }))))
}

async fn api_delete_ssh_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::ServersDelete).await?;
    state.store.delete_ssh_host(&id).await?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "ssh.host_delete",
            "server",
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
struct SshTermQuery {
    cols: Option<u32>,
    rows: Option<u32>,
}

async fn ws_ssh_terminal(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<SshTermQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::TerminalExec).await?;
    let (host, cfg) = load_ssh_config(&state, &id).await?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "ssh.terminal_open",
            "server",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "host": host.host, "username": host.username }),
        )
        .await?;

    let cols = q.cols.unwrap_or(100);
    let rows = q.rows.unwrap_or(30);
    Ok(ws.on_upgrade(move |socket| handle_ssh_ws(socket, cfg, cols, rows)))
}

async fn handle_ssh_ws(socket: WebSocket, cfg: SshConnectionConfig, cols: u32, rows: u32) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (stdin_tx, stdin_rx) = mpsc::channel::<SshInputMsg>(128);
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<Vec<u8>>(128);
    let cancel = CancellationToken::new();

    let cancel_ssh = cancel.clone();
    let err_tx = stdout_tx.clone();
    let ssh_task = tokio::spawn(async move {
        if let Err(err) = run_interactive_pty(cfg, cols, rows, stdin_rx, stdout_tx, cancel_ssh).await {
            let _ = err_tx
                .send(format!("\r\n\x1b[31mSSH error: {err}\x1b[0m\r\n").into_bytes())
                .await;
        }
    });

    let cancel_in = cancel.clone();
    let in_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(txt) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                        if v.get("type").and_then(|t| t.as_str()) == Some("resize") {
                            let c = v.get("cols").and_then(|x| x.as_u64()).unwrap_or(100) as u32;
                            let r = v.get("rows").and_then(|x| x.as_u64()).unwrap_or(30) as u32;
                            let _ = stdin_tx.send(SshInputMsg::Resize { cols: c, rows: r }).await;
                            continue;
                        }
                        if v.get("type").and_then(|t| t.as_str()) == Some("input") {
                            if let Some(data) = v.get("data").and_then(|d| d.as_str()) {
                                if stdin_tx
                                    .send(SshInputMsg::Data(data.as_bytes().to_vec()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        }
                    }
                    if stdin_tx
                        .send(SshInputMsg::Data(txt.as_bytes().to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Message::Binary(bin) => {
                    if stdin_tx.send(SshInputMsg::Data(bin.to_vec())).await.is_err() {
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
    ssh_task.abort();
}

// --- GitHub Projects & Live Compose Deploy Handlers ---

async fn api_list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<Vec<meetoops_core::ProjectDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let projects = state.store.list_projects().await?;
    Ok(Json(ApiResponse::ok(projects)))
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    ssh_host_id: String,
    repo_url: String,
    branch: Option<String>,
    deploy_path: String,
    deploy_command: Option<String>,
    compose_override: Option<String>,
    git_token: Option<String>,
    env_content: Option<String>,
}

async fn api_create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<ApiResponse<meetoops_core::ProjectDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    // Verify SSH host exists
    let _ = state.store.get_ssh_host(&payload.ssh_host_id).await?;

    let has_git_token = payload
        .git_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();
    let has_env_vars = payload
        .env_content
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();

    let project = state
        .store
        .create_project(
            &payload.name,
            &payload.ssh_host_id,
            &payload.repo_url,
            payload.branch.as_deref().unwrap_or("main"),
            &payload.deploy_path,
            payload
                .deploy_command
                .as_deref()
                .unwrap_or("docker compose up -d --build"),
            payload.compose_override.as_deref(),
            has_git_token,
            has_env_vars,
        )
        .await?;

    if let Some(tok) = payload
        .git_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        state
            .store
            .save_encrypted_secret(&state.vault, "project_git_token", &project.id, tok.as_bytes())
            .await?;
    }
    if let Some(env_txt) = payload
        .env_content
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        state
            .store
            .save_encrypted_secret(&state.vault, "project_env", &project.id, env_txt.as_bytes())
            .await?;
    }

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "project.create",
            "container",
            Some(&project.id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": project.name, "repo_url": project.repo_url }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(project)))
}

#[derive(Deserialize)]
struct UpdateProjectRequest {
    branch: Option<String>,
    deploy_command: Option<String>,
    compose_override: Option<String>,
    git_token: Option<String>,
    env_content: Option<String>,
}

async fn api_update_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<ApiResponse<meetoops_core::ProjectDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let mut has_tok = None;
    if let Some(tok) = payload.git_token.as_deref().map(str::trim) {
        if !tok.is_empty() {
            state
                .store
                .save_encrypted_secret(&state.vault, "project_git_token", &id, tok.as_bytes())
                .await?;
            has_tok = Some(true);
        }
    }
    let mut has_env = None;
    if let Some(env_txt) = payload.env_content.as_deref() {
        state
            .store
            .save_encrypted_secret(&state.vault, "project_env", &id, env_txt.as_bytes())
            .await?;
        has_env = Some(!env_txt.trim().is_empty());
    }

    let updated = state
        .store
        .update_project_config(
            &id,
            payload.branch.as_deref().unwrap_or(""),
            payload.deploy_command.as_deref().unwrap_or(""),
            payload.compose_override.as_deref(),
            has_tok,
            has_env,
        )
        .await?;
    Ok(Json(ApiResponse::ok(updated)))
}

async fn api_reveal_project_env(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerRevealSecrets).await?;
    let bytes = state
        .store
        .load_encrypted_secret(&state.vault, "project_env", &id)
        .await?;
    let content = bytes
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_default();

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "project.env_reveal",
            "container",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "env_content": content }))))
}

async fn api_delete_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    state.store.delete_project(&id).await?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "project.delete",
            "container",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "deleted": true }))))
}

async fn api_check_project_remote(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<meetoops_core::ProjectDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let project = state.store.get_project(&id).await?;
    let (_host, cfg) = load_ssh_config(&state, &project.ssh_host_id).await?;

    let token = state
        .store
        .load_encrypted_secret(&state.vault, "project_git_token", &id)
        .await?
        .map(|b| String::from_utf8_lossy(&b).to_string());
    let auth_url = inject_git_token(&project.repo_url, token.as_deref());

    let cmd = format!(
        "git ls-remote {} refs/heads/{} | awk '{{print $1}}'",
        shell_escape(&auth_url),
        shell_escape(&project.branch)
    );
    let out = exec_command(&cfg, &cmd, None, Duration::from_secs(20))
        .await
        .map_err(map_ssh_err)?;
    let remote_sha = out.stdout.trim().chars().take(12).collect::<String>();
    if !remote_sha.is_empty() {
        state
            .store
            .update_project_git_status(&id, Some(&remote_sha), None, None)
            .await?;
    }

    let refreshed = state.store.get_project(&id).await?;
    Ok(Json(ApiResponse::ok(refreshed)))
}

async fn api_deploy_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<meetoops_core::ProjectDeploymentDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let project = state.store.get_project(&id).await?;
    let (_host, cfg) = load_ssh_config(&state, &project.ssh_host_id).await?;

    let token = state
        .store
        .load_encrypted_secret(&state.vault, "project_git_token", &id)
        .await?
        .map(|b| String::from_utf8_lossy(&b).to_string());
    let env_content = state
        .store
        .load_encrypted_secret(&state.vault, "project_env", &id)
        .await?
        .map(|b| String::from_utf8_lossy(&b).to_string());

    let deploy_id = state
        .store
        .create_project_deployment(&id, "deploy", Some(&session.user.email))
        .await?;
    let _ = state
        .store
        .update_project_git_status(&id, None, None, Some("deploying"))
        .await;

    let auth_url = inject_git_token(&project.repo_url, token.as_deref());
    let path_esc = shell_escape(&project.deploy_path);
    let branch_esc = shell_escape(&project.branch);
    let url_esc = shell_escape(&auth_url);

    let mut script = format!(
        "set -e\n\
         mkdir -p {path_esc}\n\
         if [ ! -d {path_esc}/.git ]; then\n\
           git clone --branch {branch_esc} {url_esc} {path_esc}\n\
         else\n\
           cd {path_esc}\n\
           git remote set-url origin {url_esc}\n\
           git fetch origin {branch_esc}\n\
           git checkout {branch_esc}\n\
           git pull origin {branch_esc}\n\
         fi\n\
         cd {path_esc}\n"
    );

    if let Some(ref env_txt) = env_content {
        if !env_txt.trim().is_empty() {
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                env_txt.as_bytes(),
            );
            script.push_str(&format!(
                "echo '{b64}' | base64 -d > .env && chmod 600 .env\n"
            ));
        }
    }

    if let Some(ref compose_txt) = project.compose_override {
        if !compose_txt.trim().is_empty() {
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                compose_txt.as_bytes(),
            );
            script.push_str(&format!(
                "echo '{b64}' | base64 -d > docker-compose.yml\n"
            ));
        }
    }

    script.push_str(&format!(
        "{}\n\
         echo \"__MEETOOPS_COMMIT__=$(git rev-parse --short=12 HEAD)\"\n",
        project.deploy_command
    ));

    let exec_res = exec_command(&cfg, &script, None, Duration::from_secs(300)).await;
    let (status, commit_hash, log_output) = match exec_res {
        Ok(out) => {
            let mut commit = None;
            for line in out.stdout.lines() {
                if let Some(sha) = line.strip_prefix("__MEETOOPS_COMMIT__=") {
                    commit = Some(sha.trim().to_string());
                }
            }
            let combined = redact_token(
                &format!("{}\n{}", out.stdout, out.stderr),
                token.as_deref(),
            );
            let st = if out.exit_code == 0 { "success" } else { "failed" };
            (st, commit, combined)
        }
        Err(e) => (
            "failed",
            None,
            redact_token(&format!("SSH deployment error: {e}"), token.as_deref()),
        ),
    };

    state
        .store
        .finish_project_deployment(&deploy_id, status, commit_hash.as_deref(), &log_output)
        .await?;
    state
        .store
        .update_project_git_status(
            &id,
            commit_hash.as_deref(),
            commit_hash.as_deref(),
            Some(if status == "success" { "running" } else { "failed" }),
        )
        .await?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "project.deploy",
            "container",
            Some(&id),
            None,
            status,
            Some(&ip),
            serde_json::json!({ "name": project.name, "commit": commit_hash }),
        )
        .await?;

    let list = state.store.list_project_deployments(&id, 1).await?;
    let latest = list
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::bad_request("Deployment record missing."))?;
    Ok(Json(ApiResponse::ok(latest)))
}

async fn api_project_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((id, action)): Path<(String, String)>,
) -> Result<Json<ApiResponse<meetoops_core::ProjectDeploymentDto>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let cmd_suffix = match action.as_str() {
        "restart" => "docker compose restart",
        "stop" => "docker compose down",
        "logs" => "docker compose logs --tail 100",
        _ => {
            return Err(ApiError::bad_request(
                "Supported project actions: restart, stop, logs.",
            ));
        }
    };

    let project = state.store.get_project(&id).await?;
    let (_host, cfg) = load_ssh_config(&state, &project.ssh_host_id).await?;
    let deploy_id = state
        .store
        .create_project_deployment(&id, &action, Some(&session.user.email))
        .await?;

    let cmd = format!("cd {} && {cmd_suffix}", shell_escape(&project.deploy_path));
    let out = exec_command(&cfg, &cmd, None, Duration::from_secs(60))
        .await
        .map_err(map_ssh_err)?;
    let status = if out.exit_code == 0 { "success" } else { "failed" };
    let combined = format!("{}\n{}", out.stdout, out.stderr);

    state
        .store
        .finish_project_deployment(&deploy_id, status, None, &combined)
        .await?;
    if action == "stop" && status == "success" {
        let _ = state
            .store
            .update_project_git_status(&id, None, None, Some("idle"))
            .await;
    }

    let list = state.store.list_project_deployments(&id, 1).await?;
    let latest = list
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::bad_request("Action record missing."))?;
    Ok(Json(ApiResponse::ok(latest)))
}

async fn api_list_project_deployments(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<meetoops_core::ProjectDeploymentDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let list = state.store.list_project_deployments(&id, 20).await?;
    Ok(Json(ApiResponse::ok(list)))
}

#[derive(Deserialize)]
struct ScanProjectsRequest {
    ssh_host_id: String,
    base_dir: Option<String>,
}

#[derive(Serialize)]
struct ScanProjectsResponse {
    scanned_dir: String,
    found_count: usize,
    imported_count: usize,
    projects: Vec<meetoops_core::ProjectDto>,
}

fn strip_url_credentials(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some((_creds, host_part)) = rest.split_once('@') {
            return format!("https://{host_part}");
        }
    }
    url.to_string()
}

/// Scans an existing directory on the target SSH host (e.g. `/home/app`) for Git repositories,
/// extracts origin URL, active branch, and HEAD commit, and imports any new ones into Projects.
async fn api_scan_existing_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<ScanProjectsRequest>,
) -> Result<Json<ApiResponse<ScanProjectsResponse>>, ApiError> {
    let session = require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let (_host, cfg) = load_ssh_config(&state, &payload.ssh_host_id).await?;
    let base_dir = payload
        .base_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("/home/app");

    let script = format!(
        "for g in $(find {} -maxdepth 3 -name .git -type d 2>/dev/null); do \
           dir=$(dirname \"$g\"); \
           name=$(basename \"$dir\"); \
           url=$(git -C \"$dir\" remote get-url origin 2>/dev/null || echo ''); \
           branch=$(git -C \"$dir\" rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'main'); \
           commit=$(git -C \"$dir\" rev-parse --short=12 HEAD 2>/dev/null || echo ''); \
           has_env=0; [ -f \"$dir/.env\" ] && has_env=1; \
           echo \"$name|$dir|$url|$branch|$commit|$has_env\"; \
         done",
        shell_escape(base_dir)
    );

    let out = exec_command(&cfg, &script, None, Duration::from_secs(25))
        .await
        .map_err(map_ssh_err)?;

    let existing = state.store.list_projects().await?;
    let mut found_count = 0usize;
    let mut imported_count = 0usize;

    for line in out.stdout.lines() {
        let parts: Vec<&str> = line.trim().split('|').collect();
        if parts.len() < 6 {
            continue;
        }
        let name = parts[0].trim();
        let dir = parts[1].trim();
        let raw_url = parts[2].trim();
        let branch = parts[3].trim();
        let commit = parts[4].trim();
        let has_env = parts[5].trim() == "1";

        if name.is_empty() || dir.is_empty() || raw_url.is_empty() {
            continue;
        }
        found_count += 1;

        let clean_url = strip_url_credentials(raw_url);
        if let Some(prev) = existing
            .iter()
            .find(|p| p.deploy_path == dir && p.ssh_host_id == payload.ssh_host_id)
        {
            if !commit.is_empty() {
                let _ = state
                    .store
                    .update_project_git_status(&prev.id, None, Some(commit), None)
                    .await;
            }
            continue;
        }

        // Ensure unique project name if same folder name exists on another host
        let unique_name = if existing.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
            format!("{name}-{}", &payload.ssh_host_id.chars().take(6).collect::<String>())
        } else {
            name.to_string()
        };

        if let Ok(created) = state
            .store
            .create_project(
                &unique_name,
                &payload.ssh_host_id,
                &clean_url,
                if branch.is_empty() || branch == "HEAD" { "main" } else { branch },
                dir,
                "docker compose up -d --build",
                None,
                false,
                has_env,
            )
            .await
        {
            if !commit.is_empty() {
                let _ = state
                    .store
                    .update_project_git_status(&created.id, None, Some(commit), Some("running"))
                    .await;
            }
            imported_count += 1;
        }
    }

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "project.scan_existing",
            "container",
            Some(&payload.ssh_host_id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "base_dir": base_dir, "found": found_count, "imported": imported_count }),
        )
        .await?;

    let projects = state.store.list_projects().await?;
    Ok(Json(ApiResponse::ok(ScanProjectsResponse {
        scanned_dir: base_dir.to_string(),
        found_count,
        imported_count,
        projects,
    })))
}
