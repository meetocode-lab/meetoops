use meetoops_common::DEFAULT_LISTEN_PORT;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;

/// Deployment-level bootstrap configuration.
/// Zero-config by default: requires NO `.env` file.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub key_path: PathBuf,
    pub static_dir: PathBuf,
    pub bind_host: String,
    pub env_port_override: Option<u16>,
    pub trust_proxy: bool,
}

impl BootstrapConfig {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("MEETOOPS_DATA_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let container_data = PathBuf::from("/data");
                if container_data.exists() {
                    container_data
                } else {
                    PathBuf::from(".meetoops-dev")
                }
            });

        let db_path = data_dir.join("db").join("meetoops.db");
        let key_path = std::env::var("MEETOOPS_KEY_PATH")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("keys").join("master.key"));

        let static_dir = std::env::var("MEETOOPS_WEB_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let prod_web = PathBuf::from("/app/web");
                if prod_web.exists() {
                    prod_web
                } else {
                    PathBuf::from("apps/web/dist")
                }
            });

        let mut bind_host = "0.0.0.0".to_string();
        let mut env_port_override = None;

        if let Ok(bind_addr) = std::env::var("MEETOOPS_BIND_ADDR") {
            if let Some((h, p)) = bind_addr.rsplit_once(':') {
                if !h.is_empty() {
                    bind_host = h.to_string();
                }
                if let Ok(port) = p.parse::<u16>() {
                    env_port_override = Some(port);
                }
            }
        }
        if let Ok(port_str) = std::env::var("MEETOOPS_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                env_port_override = Some(port);
            }
        }

        let trust_proxy = std::env::var("MEETOOPS_TRUST_PROXY")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        Self {
            data_dir,
            db_path,
            key_path,
            static_dir,
            bind_host,
            env_port_override,
            trust_proxy,
        }
    }

    pub fn resolve_listen_port(&self, db_configured_port: Option<u16>) -> u16 {
        self.env_port_override
            .or(db_configured_port)
            .unwrap_or(DEFAULT_LISTEN_PORT)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PortCheckResult {
    pub port: u16,
    pub available: bool,
    pub is_current_port: bool,
    pub reason: String,
    pub suggested_ports: Vec<u16>,
}

/// Performs a real OS-level TCP bind probe to determine if `port` is available on the host.
pub fn check_port_availability(port: u16, current_active_port: u16) -> PortCheckResult {
    if port < 1024 {
        return PortCheckResult {
            port,
            available: false,
            is_current_port: port == current_active_port,
            reason: "Privileged ports below 1024 are restricted. Choose a port between 1024 and 65535.".to_string(),
            suggested_ports: find_suggested_ports(9480, current_active_port),
        };
    }

    if port == current_active_port {
        return PortCheckResult {
            port,
            available: true,
            is_current_port: true,
            reason: "This port is currently active and bound by MeetoOps.".to_string(),
            suggested_ports: vec![],
        };
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    match TcpListener::bind(addr) {
        Ok(listener) => {
            drop(listener);
            PortCheckResult {
                port,
                available: true,
                is_current_port: false,
                reason: format!("Port {port} is available and ready to use."),
                suggested_ports: vec![],
            }
        }
        Err(err) => PortCheckResult {
            port,
            available: false,
            is_current_port: false,
            reason: format!("Port {port} is already in use or cannot be bound ({err})."),
            suggested_ports: find_suggested_ports(port.saturating_add(1), current_active_port),
        },
    }
}

fn find_suggested_ports(start: u16, current_active_port: u16) -> Vec<u16> {
    let mut found = Vec::new();
    let mut candidate = start.max(1024);
    for _ in 0..200 {
        if found.len() >= 3 {
            break;
        }
        if candidate != current_active_port {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), candidate);
            if let Ok(listener) = TcpListener::bind(addr) {
                drop(listener);
                found.push(candidate);
            }
        }
        candidate = if candidate >= 65530 { 9481 } else { candidate + 1 };
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bound_port_collision() {
        let occupied = TcpListener::bind("0.0.0.0:0").expect("bind ephemeral");
        let occupied_port = occupied.local_addr().unwrap().port();

        let res = check_port_availability(occupied_port, 9480);
        assert!(!res.available);
        assert!(!res.suggested_ports.is_empty());

        drop(occupied);
    }

    #[test]
    fn rejects_privileged_ports() {
        let res = check_port_availability(80, 9480);
        assert!(!res.available);
    }
}
