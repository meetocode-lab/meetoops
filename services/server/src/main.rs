mod auth;
mod docker_routes;
mod error;
mod explorer_routes;
mod monitoring_routes;
mod ops_routes;
mod phase5_routes;
mod routes;
mod ssh_projects_routes;
mod state;

use axum::http::{header, HeaderValue};
use axum::Router;
use meetoops_core::{init_database, BootstrapConfig, CoreStore};
use meetoops_security::MasterKeyVault;
use state::AppState;
use std::net::SocketAddr;
use std::path::Path;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

pub fn build_app(state: AppState, static_dir: &Path) -> Router {
    let api = routes::api_router()
        .merge(docker_routes::docker_router())
        .merge(monitoring_routes::monitoring_router())
        .merge(explorer_routes::explorer_router())
        .merge(ops_routes::ops_router())
        .merge(phase5_routes::phase5_router())
        .merge(ssh_projects_routes::ssh_projects_router());
    let index_file = static_dir.join("index.html");

    let app = if index_file.exists() {
        let spa = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_file));
        api.fallback_service(spa)
    } else {
        api
    };

    app.layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:; img-src 'self' data: blob:; connect-src 'self' ws: wss:; frame-ancestors 'none'",
            ),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Minimal built-in healthcheck subcommand for distroless/slim Docker containers
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let cfg = BootstrapConfig::from_env();
        let port = cfg.resolve_listen_port(None);
        match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
            Ok(_) => std::process::exit(0),
            Err(_) => std::process::exit(1),
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,meetoops_server=info,tower_http=info")),
        )
        .init();

    let config = BootstrapConfig::from_env();
    tracing::info!(
        data_dir = %config.data_dir.display(),
        "bootstrapping MeetoOps Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    let vault = MasterKeyVault::load_or_create(&config.key_path)?;
    let pool = init_database(&config.db_path, &vault).await?;
    let store = CoreStore::new(pool);

    let settings = store.get_settings().await.unwrap_or_default();
    let listen_port = config.resolve_listen_port(Some(settings.listen_port));

    let app_state = AppState::new(store, vault, config.clone(), listen_port);
    monitoring_routes::spawn_metrics_collector(app_state.clone());
    let router = build_app(app_state, &config.static_dir);

    let bind_str = format!("{}:{}", config.bind_host, listen_port);
    let addr: SocketAddr = bind_str.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "MeetoOps Control Center listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("MeetoOps Server shut down cleanly");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use meetoops_core::db::init_in_memory_database;
    use tower::ServiceExt;

    #[tokio::test]
    async fn end_to_end_setup_auth_and_port_check() {
        let vault = MasterKeyVault::from_bytes([11u8; 32]);
        let pool = init_in_memory_database(&vault).await.expect("in-mem db");
        let store = CoreStore::new(pool);
        let cfg = BootstrapConfig::from_env();
        let state = AppState::new(store, vault, cfg, 9480);
        let app = build_app(state, Path::new("nonexistent"));

        // 1. Setup status is FIRST_RUN_SETUP
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/setup/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. Unauthenticated dashboard returns 401
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/dashboard/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // 3. Create initial super admin
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/setup/admin")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "full_name": "Meeto Admin",
                            "email": "admin@meetoops.io",
                            "password": "WarmObsidian2026!"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let set_cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let csrf = parsed["data"]["csrf_token"].as_str().unwrap().to_string();

        // 4. Complete setup locks setup permanently
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/setup/complete")
                    .header(header::COOKIE, &set_cookie)
                    .header("x-csrf-token", &csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 5. Re-running setup/admin fails with 403 SETUP_LOCKED
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/setup/admin")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "full_name": "Hacker",
                            "email": "hacker@evil.com",
                            "password": "WarmObsidian2026!"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // 6. Real port availability probe succeeds for authenticated admin
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/settings/network/check-port")
                    .header("content-type", "application/json")
                    .header(header::COOKIE, &set_cookie)
                    .header("x-csrf-token", &csrf)
                    .body(Body::from(serde_json::json!({ "port": 9480 }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
