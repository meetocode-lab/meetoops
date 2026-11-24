use meetoops_docker::probe_local_docker;
use meetoops_monitoring::HostMetricsCollector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[derive(Serialize, Deserialize)]
struct PersistedAgentCreds {
    server_url: String,
    server_id: String,
    agent_secret: String,
}

#[derive(Deserialize)]
struct RegisterEnvelope {
    data: RegisterData,
}

#[derive(Deserialize)]
struct RegisterData {
    server_id: String,
    agent_secret: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let server_url = std::env::var("MEETOOPS_SERVER_URL")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    let reg_token = std::env::var("MEETOOPS_REGISTRATION_TOKEN").unwrap_or_default();
    let creds_path = std::env::var("MEETOOPS_AGENT_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".meetoops-agent.json"));

    if server_url.is_empty() && !creds_path.exists() {
        tracing::error!(
            "MEETOOPS_SERVER_URL and MEETOOPS_REGISTRATION_TOKEN are required on first run."
        );
        std::process::exit(1);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    let creds: PersistedAgentCreds = if creds_path.exists() {
        let raw = std::fs::read_to_string(&creds_path).expect("read agent state");
        serde_json::from_str(&raw).expect("parse agent state")
    } else {
        tracing::info!(%server_url, "registering meetoops-agent with control plane...");
        let resp = client
            .post(format!("{server_url}/api/v1/agent/register"))
            .json(&serde_json::json!({ "registration_token": reg_token }))
            .send()
            .await
            .expect("outbound registration request");

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::error!(%status, body = %text, "agent registration failed");
            std::process::exit(1);
        }

        let env: RegisterEnvelope = resp.json().await.expect("parse registration response");
        let saved = PersistedAgentCreds {
            server_url: server_url.clone(),
            server_id: env.data.server_id,
            agent_secret: env.data.agent_secret,
        };
        let _ = std::fs::write(
            &creds_path,
            serde_json::to_string_pretty(&saved).unwrap_or_default(),
        );
        tracing::info!(server_id = %saved.server_id, "agent registered and credentials persisted");
        saved
    };

    let interval_secs: u64 = std::env::var("MEETOOPS_HEARTBEAT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15)
        .clamp(5, 300);

    tracing::info!(
        server_id = %creds.server_id,
        interval_secs,
        "meetoops-agent v{} online — starting outbound heartbeat loop",
        env!("CARGO_PKG_VERSION")
    );

    let collector = Arc::new(Mutex::new(HostMetricsCollector::new()));

    loop {
        let c = collector.clone();
        let host = tokio::task::spawn_blocking(move || {
            let mut guard = c.lock().expect("agent metrics collector mutex");
            guard.sample()
        })
        .await
        .ok();
        let docker = probe_local_docker().await;

        let metrics_payload = host.as_ref().map(|h| {
            serde_json::json!({
                "cpu_percent": h.cpu_usage_percent.map(|v| v as f64),
                "mem_used_bytes": h.memory_used_bytes.map(|v| v as i64),
                "mem_total_bytes": h.memory_total_bytes.map(|v| v as i64),
                "mem_percent": h.memory_usage_percent.map(|v| v as f64),
                "disk_used_bytes": h.disk_used_bytes.map(|v| v as i64),
                "disk_total_bytes": h.disk_total_bytes.map(|v| v as i64),
                "disk_percent": h.disk_usage_percent.map(|v| v as f64),
                "disk_active_percent": h.disk_active_percent.map(|v| v as f64),
                "disk_read_bps": h.disk_read_bps,
                "disk_write_bps": h.disk_write_bps,
                "net_rx_bps": h.net_rx_bps,
                "net_tx_bps": h.net_tx_bps,
                "uptime_seconds": h.uptime_seconds.map(|v| v as i64),
            })
        });

        let payload = serde_json::json!({
            "server_id": creds.server_id,
            "agent_secret": creds.agent_secret,
            "docker_version": docker.docker_version,
            "os_info": host.as_ref().and_then(|h| h.os_name.clone()),
            "cpu_cores": host.as_ref().map(|h| h.cpu_cores as i64),
            "memory_total_bytes": host.as_ref().and_then(|h| h.memory_total_bytes.map(|b| b as i64)),
            "agent_version": env!("CARGO_PKG_VERSION"),
            "metrics": metrics_payload,
        });

        match client
            .post(format!("{}/api/v1/agent/heartbeat", creds.server_url))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!(server_id = %creds.server_id, "heartbeat acknowledged");
            }
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "heartbeat rejected by control plane");
            }
            Err(err) => {
                tracing::warn!(error = %err, "outbound heartbeat failed; will retry");
            }
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}
