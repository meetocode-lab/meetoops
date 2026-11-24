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
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use meetoops_common::{ApiResponse, PaginationMeta};
use meetoops_docker::{
    container_action, container_logs_snapshot, container_stats_snapshot, create_network,
    create_volume, inspect_container, list_containers, list_images, list_networks, list_volumes,
    prune_images, prune_networks, prune_volumes, pull_image, remove_container, remove_image,
    remove_network, remove_volume, stream_container_logs, DockerError, LogLineDto,
};
use meetoops_security::Permission;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn map_docker_err(err: DockerError) -> ApiError {
    match err {
        DockerError::Unavailable(msg) => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "DOCKER_UNAVAILABLE",
            format!("Docker Engine is not reachable: {msg}"),
        ),
        DockerError::Validation(msg) => ApiError::bad_request(msg),
        DockerError::Api(e) => ApiError::new(
            StatusCode::BAD_GATEWAY,
            "DOCKER_API_ERROR",
            format!("Docker operation failed: {e}"),
        ),
    }
}

pub fn docker_router() -> Router<AppState> {
    Router::new()
        // Containers
        .route("/api/v1/docker/containers", get(api_list_containers))
        .route(
            "/api/v1/docker/containers/{id}",
            get(api_inspect_container).delete(api_remove_container),
        )
        .route(
            "/api/v1/docker/containers/{id}/env/reveal",
            post(api_reveal_container_env),
        )
        .route(
            "/api/v1/docker/containers/{id}/stats",
            get(api_container_stats),
        )
        .route(
            "/api/v1/docker/containers/{id}/{action}",
            post(api_container_action),
        )
        .route("/api/v1/docker/containers/{id}/logs", get(api_container_logs))
        .route(
            "/ws/v1/docker/containers/{id}/logs",
            get(ws_container_logs),
        )
        // Images
        .route("/api/v1/docker/images", get(api_list_images))
        .route("/api/v1/docker/images/pull", post(api_pull_image))
        .route("/api/v1/docker/images/prune", post(api_prune_images))
        .route("/api/v1/docker/images/{id}", delete(api_remove_image))
        // Volumes
        .route(
            "/api/v1/docker/volumes",
            get(api_list_volumes).post(api_create_volume),
        )
        .route("/api/v1/docker/volumes/prune", post(api_prune_volumes))
        .route("/api/v1/docker/volumes/{name}", delete(api_remove_volume))
        // Networks
        .route(
            "/api/v1/docker/networks",
            get(api_list_networks).post(api_create_network),
        )
        .route("/api/v1/docker/networks/prune", post(api_prune_networks))
        .route("/api/v1/docker/networks/{id}", delete(api_remove_network))
}

#[derive(Deserialize)]
struct ListContainersQuery {
    all: Option<bool>,
    search: Option<String>,
    state: Option<String>, // comma-separated: running,exited,paused
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn api_list_containers(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<ListContainersQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::ContainerSummaryDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let mut list = list_containers(q.all.unwrap_or(true))
        .await
        .map_err(map_docker_err)?;

    let states: Vec<String> = q
        .state
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty() && s != "all")
        .collect();
    if !states.is_empty() {
        list.retain(|c| states.iter().any(|st| st == &c.state.to_ascii_lowercase()));
    }
    if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = search.to_ascii_lowercase();
        list.retain(|c| {
            c.name.to_ascii_lowercase().contains(&needle)
                || c.image.to_ascii_lowercase().contains(&needle)
                || c.short_id.to_ascii_lowercase().contains(&needle)
        });
    }

    let asc = q
        .sort_dir
        .as_deref()
        .map(|d| d.eq_ignore_ascii_case("asc"))
        .unwrap_or(true);
    match q.sort_by.as_deref().unwrap_or("name") {
        "image" => list.sort_by(|a, b| a.image.cmp(&b.image)),
        "state" => list.sort_by(|a, b| a.state.cmp(&b.state)),
        "created" => list.sort_by(|a, b| a.created.cmp(&b.created)),
        _ => list.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    if !asc {
        list.reverse();
    }

    let total = list.len() as u64;
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);
    let start = ((page - 1) as usize).saturating_mul(per_page as usize);
    let paged = list
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    Ok(Json(ApiResponse::paginated(
        paged,
        PaginationMeta::new(page, per_page, total),
    )))
}

async fn api_inspect_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<meetoops_docker::ContainerInspectDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let info = inspect_container(&id, false)
        .await
        .map_err(map_docker_err)?;
    Ok(Json(ApiResponse::ok(info)))
}

async fn api_container_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<meetoops_docker::ContainerStatsSnapshotDto>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let stats = container_stats_snapshot(&id)
        .await
        .map_err(map_docker_err)?;
    Ok(Json(ApiResponse::ok(stats)))
}

async fn api_reveal_container_env(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::EnvVarDto>>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerRevealSecrets).await?;
    let info = inspect_container(&id, true).await.map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "container.env_reveal",
            "container",
            Some(&info.short_id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "container_name": info.name }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(info.env)))
}

async fn api_container_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((id, action)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    container_action(&id, &action)
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            &format!("container.{action}"),
            "container",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "ok": true, "action": action }),
    )))
}

#[derive(Deserialize)]
struct RemoveContainerQuery {
    force: Option<bool>,
    volumes: Option<bool>,
}

async fn api_remove_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<RemoveContainerQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    let force = q.force.unwrap_or(false);
    let vols = q.volumes.unwrap_or(false);
    remove_container(&id, force, vols)
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "container.remove",
            "container",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "force": force, "remove_volumes": vols }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "removed": true }))))
}

#[derive(Deserialize)]
struct LogsQuery {
    tail: Option<usize>,
    timestamps: Option<bool>,
}

async fn api_container_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<ApiResponse<Vec<LogLineDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::LogsRead).await?;
    let lines = container_logs_snapshot(&id, q.tail.unwrap_or(200), q.timestamps.unwrap_or(true))
        .await
        .map_err(map_docker_err)?;
    Ok(Json(ApiResponse::ok(lines)))
}

async fn ws_container_logs(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::LogsRead).await?;
    let tail = q.tail.unwrap_or(200);
    let timestamps = q.timestamps.unwrap_or(true);
    Ok(ws.on_upgrade(move |socket| handle_logs_ws(socket, id, tail, timestamps)))
}

async fn handle_logs_ws(socket: WebSocket, container_id: String, tail: usize, timestamps: bool) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::channel::<LogLineDto>(256);
    let cancel = CancellationToken::new();
    let paused = Arc::new(AtomicBool::new(false));

    let cancel_reader = cancel.clone();
    let cid = container_id.clone();
    tokio::spawn(async move {
        if let Err(err) = stream_container_logs(cid, tail, timestamps, tx, cancel_reader).await {
            tracing::warn!(error = %err, "container log stream ended with error");
        }
    });

    let paused_ctrl = paused.clone();
    let cancel_ctrl = cancel.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(txt) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                        match v.get("type").and_then(|t| t.as_str()) {
                            Some("pause") => paused_ctrl.store(true, Ordering::Relaxed),
                            Some("resume") => paused_ctrl.store(false, Ordering::Relaxed),
                            _ => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        cancel_ctrl.cancel();
    });

    while let Some(line) = rx.recv().await {
        if paused.load(Ordering::Relaxed) {
            continue;
        }
        if let Ok(json) = serde_json::to_string(&line) {
            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }

    cancel.cancel();
    recv_task.abort();
}

#[derive(Deserialize)]
struct DockerTableQuery {
    search: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn api_list_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<DockerTableQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::ImageSummaryDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let mut images = list_images().await.map_err(map_docker_err)?;

    if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = search.to_ascii_lowercase();
        images.retain(|img| {
            img.short_id.to_ascii_lowercase().contains(&needle)
                || img
                    .repo_tags
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains(&needle))
        });
    }

    let asc = q
        .sort_dir
        .as_deref()
        .map(|d| d.eq_ignore_ascii_case("asc"))
        .unwrap_or(false);
    match q.sort_by.as_deref().unwrap_or("created") {
        "tag" => images.sort_by(|a, b| {
            let ta = a.repo_tags.first().map(String::as_str).unwrap_or("");
            let tb = b.repo_tags.first().map(String::as_str).unwrap_or("");
            ta.cmp(tb)
        }),
        "size" => images.sort_by(|a, b| a.size_bytes.cmp(&b.size_bytes)),
        _ => images.sort_by(|a, b| a.created.cmp(&b.created)),
    }
    if !asc {
        images.reverse();
    }

    let total = images.len() as u64;
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 200);
    let start = ((page - 1) as usize).saturating_mul(per_page as usize);
    let paged = images
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    Ok(Json(ApiResponse::paginated(
        paged,
        PaginationMeta::new(page, per_page, total),
    )))
}

#[derive(Deserialize)]
struct PullImageRequest {
    image: String,
}

async fn api_pull_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<PullImageRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let statuses = pull_image(&payload.image).await.map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "image.pull",
            "image",
            Some(&payload.image),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "steps": statuses.len() }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "image": payload.image, "statuses": statuses }),
    )))
}

#[derive(Deserialize)]
struct ForceQuery {
    force: Option<bool>,
}

async fn api_remove_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
    Query(q): Query<ForceQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    remove_image(&id, q.force.unwrap_or(false))
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "image.remove",
            "image",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "force": q.force.unwrap_or(false) }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "removed": true }))))
}

async fn api_list_volumes(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<DockerTableQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::VolumeSummaryDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let mut volumes = list_volumes().await.map_err(map_docker_err)?;

    if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = search.to_ascii_lowercase();
        volumes.retain(|v| {
            v.name.to_ascii_lowercase().contains(&needle)
                || v.driver.to_ascii_lowercase().contains(&needle)
                || v.mountpoint.to_ascii_lowercase().contains(&needle)
        });
    }

    let asc = q
        .sort_dir
        .as_deref()
        .map(|d| d.eq_ignore_ascii_case("asc"))
        .unwrap_or(true);
    match q.sort_by.as_deref().unwrap_or("name") {
        "driver" => volumes.sort_by(|a, b| a.driver.cmp(&b.driver)),
        "created_at" => volumes.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        _ => volumes.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    if !asc {
        volumes.reverse();
    }

    let total = volumes.len() as u64;
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 200);
    let start = ((page - 1) as usize).saturating_mul(per_page as usize);
    let paged = volumes
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    Ok(Json(ApiResponse::paginated(
        paged,
        PaginationMeta::new(page, per_page, total),
    )))
}

#[derive(Deserialize)]
struct CreateVolumeRequest {
    name: String,
    driver: Option<String>,
}

async fn api_create_volume(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateVolumeRequest>,
) -> Result<Json<ApiResponse<meetoops_docker::VolumeSummaryDto>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let vol = create_volume(&payload.name, payload.driver.as_deref())
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "volume.create",
            "volume",
            Some(&vol.name),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "driver": vol.driver }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(vol)))
}

async fn api_remove_volume(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(name): Path<String>,
    Query(q): Query<ForceQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    remove_volume(&name, q.force.unwrap_or(false))
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "volume.remove",
            "volume",
            Some(&name),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "removed": true }))))
}

async fn api_list_networks(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Query(q): Query<DockerTableQuery>,
) -> Result<Json<ApiResponse<Vec<meetoops_docker::NetworkSummaryDto>>>, ApiError> {
    let _session = require_permission(&state, &headers, &method, Permission::DockerRead).await?;
    let mut nets = list_networks().await.map_err(map_docker_err)?;

    if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = search.to_ascii_lowercase();
        nets.retain(|n| {
            n.name.to_ascii_lowercase().contains(&needle)
                || n.driver.to_ascii_lowercase().contains(&needle)
                || n.short_id.to_ascii_lowercase().contains(&needle)
        });
    }

    let asc = q
        .sort_dir
        .as_deref()
        .map(|d| d.eq_ignore_ascii_case("asc"))
        .unwrap_or(true);
    match q.sort_by.as_deref().unwrap_or("name") {
        "driver" => nets.sort_by(|a, b| a.driver.cmp(&b.driver)),
        "scope" => nets.sort_by(|a, b| a.scope.cmp(&b.scope)),
        _ => nets.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    if !asc {
        nets.reverse();
    }

    let total = nets.len() as u64;
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 200);
    let start = ((page - 1) as usize).saturating_mul(per_page as usize);
    let paged = nets
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    Ok(Json(ApiResponse::paginated(
        paged,
        PaginationMeta::new(page, per_page, total),
    )))
}

#[derive(Deserialize)]
struct CreateNetworkRequest {
    name: String,
    driver: Option<String>,
}

async fn api_create_network(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Json(payload): Json<CreateNetworkRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerOperate).await?;
    let id = create_network(&payload.name, payload.driver.as_deref())
        .await
        .map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "network.create",
            "network",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "name": payload.name }),
        )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "id": id, "name": payload.name }),
    )))
}

async fn api_remove_network(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    remove_network(&id).await.map_err(map_docker_err)?;

    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "network.remove",
            "network",
            Some(&id),
            None,
            "success",
            Some(&ip),
            serde_json::json!({}),
        )
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "removed": true }))))
}


async fn api_prune_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<meetoops_docker::PruneResultDto>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    let res = prune_images().await.map_err(map_docker_err)?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "image.prune",
            "image",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({
                "deleted_count": res.deleted_count,
                "space_reclaimed_bytes": res.space_reclaimed_bytes
            }),
        )
        .await?;
    Ok(Json(ApiResponse::ok(res)))
}

async fn api_prune_volumes(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<meetoops_docker::PruneResultDto>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    let res = prune_volumes().await.map_err(map_docker_err)?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "volume.prune",
            "volume",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({
                "deleted_count": res.deleted_count,
                "space_reclaimed_bytes": res.space_reclaimed_bytes
            }),
        )
        .await?;
    Ok(Json(ApiResponse::ok(res)))
}

async fn api_prune_networks(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
) -> Result<Json<ApiResponse<meetoops_docker::PruneResultDto>>, ApiError> {
    let session =
        require_permission(&state, &headers, &method, Permission::DockerDelete).await?;
    let res = prune_networks().await.map_err(map_docker_err)?;
    let ip = client_ip(&headers, state.config.trust_proxy);
    state
        .store
        .write_audit_log(
            Some(&session.user.id),
            Some(&session.user.email),
            "network.prune",
            "network",
            None,
            None,
            "success",
            Some(&ip),
            serde_json::json!({ "deleted_count": res.deleted_count }),
        )
        .await?;
    Ok(Json(ApiResponse::ok(res)))
}
