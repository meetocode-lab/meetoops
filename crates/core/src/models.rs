use meetoops_common::{Role, SetupStage, DEFAULT_LISTEN_PORT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct AppStateRecord {
    pub setup_stage: SetupStage,
    pub initialized_at: Option<String>,
    pub key_fingerprint_short: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub username: Option<String>,
    pub role: Role,
    pub is_active: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub session_id: String,
    pub csrf_token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingsDto {
    pub app_name: String,
    pub app_url: String,
    pub language: String,
    pub timezone: String,
    pub date_format: String,
    pub time_format: String,
    pub monitoring_interval_secs: u32,
    pub session_timeout_hours: u32,
    pub max_login_attempts: u32,
    pub listen_port: u16,
    pub audit_retention_days: u32,
}

impl Default for AppSettingsDto {
    fn default() -> Self {
        Self {
            app_name: "MeetoOps".to_string(),
            app_url: format!("http://localhost:{DEFAULT_LISTEN_PORT}"),
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            date_format: "YYYY-MM-DD".to_string(),
            time_format: "24h".to_string(),
            monitoring_interval_secs: 5,
            session_timeout_hours: 24,
            max_login_attempts: 5,
            listen_port: DEFAULT_LISTEN_PORT,
            audit_retention_days: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub group_name: String,
    pub connection_type: String,
    pub endpoint: Option<String>,
    pub status: String,
    pub docker_version: Option<String>,
    pub os_info: Option<String>,
    pub cpu_cores: Option<i64>,
    pub memory_total_bytes: Option<i64>,
    pub last_seen_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub actor_user_id: Option<String>,
    pub actor_email: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub server_id: Option<String>,
    pub status: String,
    pub ip_address: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostMetricPoint {
    pub server_id: String,
    pub cpu_percent: Option<f64>,
    pub mem_used_bytes: Option<i64>,
    pub mem_total_bytes: Option<i64>,
    pub mem_percent: Option<f64>,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
    pub disk_percent: Option<f64>,
    pub disk_active_percent: Option<f64>,
    pub disk_read_bps: Option<f64>,
    pub disk_write_bps: Option<f64>,
    pub net_rx_bps: Option<f64>,
    pub net_tx_bps: Option<f64>,
    pub uptime_seconds: Option<i64>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredServiceDto {
    pub id: String,
    pub server_id: String,
    pub container_id: Option<String>,
    pub name: String,
    pub service_type: String,
    pub confidence: String,
    pub readiness_stage: String,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub has_credentials: bool,
    pub read_only_mode: bool,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertRuleDto {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub metric_type: String,
    pub operator: String,
    pub threshold: f64,
    pub severity: String,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertEventDto {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub server_id: String,
    pub status: String,
    pub severity: String,
    pub message: String,
    pub metric_value: Option<f64>,
    pub triggered_at: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiTokenDto {
    pub id: String,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertChannelDto {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub target_url: String,
    pub is_enabled: bool,
    pub created_at: String,
}
