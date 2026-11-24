use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceConfidence {
    Confirmed,
    Probable,
    Unknown,
}

impl ServiceConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Probable => "probable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStage {
    Detected,
    ConnectionDiscovered,
    CredentialsAvailable,
    AuthVerified,
    ExplorerReady,
}

impl ReadinessStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::ConnectionDiscovered => "connection_discovered",
            Self::CredentialsAvailable => "credentials_available",
            Self::AuthVerified => "auth_verified",
            Self::ExplorerReady => "explorer_ready",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContainerMetadataInput {
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub exposed_ports: Vec<(u16, Option<u16>)>, // (private_port, public_port)
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedServiceCandidate {
    pub container_id: String,
    pub name: String,
    pub service_type: String,
    pub confidence: ServiceConfidence,
    pub readiness_stage: ReadinessStage,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub signals: Vec<String>,
}

/// Inspects container image, tags, ports, and labels to detect well-known infrastructure services.
/// Detection NEVER implies database access — readiness stops at `Detected` or `ConnectionDiscovered`
/// until the user explicitly supplies encrypted credentials and verifies authentication.
pub fn detect_service(meta: &ContainerMetadataInput) -> Option<DetectedServiceCandidate> {
    let img = meta.image.to_ascii_lowercase();
    let mut signals = Vec::new();

    let rules: &[(&str, &[&str], u16)] = &[
        ("postgres", &["postgres", "postgis", "timescaledb", "bitnami/postgresql"], 5432),
        ("mariadb", &["mariadb", "bitnami/mariadb"], 3306),
        ("mysql", &["mysql", "percona", "bitnami/mysql"], 3306),
        ("redis", &["redis", "valkey", "dragonfly", "bitnami/redis"], 6379),
        ("mongodb", &["mongo", "mongodb", "bitnami/mongodb"], 27017),
        ("rabbitmq", &["rabbitmq", "bitnami/rabbitmq"], 5672),
        ("elasticsearch", &["elasticsearch", "opensearch"], 9200),
        ("nginx", &["nginx", "openresty"], 80),
        ("apache", &["httpd", "apache"], 80),
    ];

    for (svc_type, image_patterns, default_port) in rules {
        let image_match = image_patterns.iter().any(|pat| img.contains(pat));
        let port_match = meta
            .exposed_ports
            .iter()
            .find(|(priv_p, _)| *priv_p == *default_port);

        if image_match || port_match.is_some() {
            if image_match {
                signals.push(format!("image matches '{svc_type}' ({})", meta.image));
            }
            if let Some((priv_p, pub_p)) = port_match {
                signals.push(format!("port {priv_p} exposed (host: {pub_p:?})"));
            }

            let confidence = if image_match && port_match.is_some() {
                ServiceConfidence::Confirmed
            } else if image_match {
                ServiceConfidence::Confirmed
            } else {
                ServiceConfidence::Probable
            };

            let default_host = if std::path::Path::new("/.dockerenv").exists() {
                "host.docker.internal".to_string()
            } else {
                "127.0.0.1".to_string()
            };

            let (host, port, readiness_stage) = if let Some((_, Some(pub_p))) = port_match {
                (
                    Some(default_host),
                    Some(*pub_p),
                    ReadinessStage::ConnectionDiscovered,
                )
            } else if let Some((priv_p, None)) = port_match {
                (
                    Some(default_host),
                    Some(*priv_p),
                    ReadinessStage::ConnectionDiscovered,
                )
            } else {
                (None, Some(*default_port), ReadinessStage::Detected)
            };

            return Some(DetectedServiceCandidate {
                container_id: meta.container_id.clone(),
                name: meta.name.clone(),
                service_type: (*svc_type).to_string(),
                confidence,
                readiness_stage,
                host,
                port,
                signals,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_postgres_and_redis_with_confidence() {
        let pg = detect_service(&ContainerMetadataInput {
            container_id: "abc123".to_string(),
            name: "prod-pg".to_string(),
            image: "postgres:16-alpine".to_string(),
            exposed_ports: vec![(5432, Some(5432))],
            labels: HashMap::new(),
        })
        .expect("detect postgres");
        assert_eq!(pg.service_type, "postgres");
        assert_eq!(pg.confidence, ServiceConfidence::Confirmed);
        assert_eq!(pg.readiness_stage, ReadinessStage::ConnectionDiscovered);
        assert_eq!(pg.port, Some(5432));

        let unknown_port_redis = detect_service(&ContainerMetadataInput {
            container_id: "def456".to_string(),
            name: "cache".to_string(),
            image: "custom/cache:latest".to_string(),
            exposed_ports: vec![(6379, Some(16379))],
            labels: HashMap::new(),
        })
        .expect("detect redis by port");
        assert_eq!(unknown_port_redis.service_type, "redis");
        assert_eq!(unknown_port_redis.confidence, ServiceConfidence::Probable);
        assert_eq!(unknown_port_redis.port, Some(16379));
    }
}
