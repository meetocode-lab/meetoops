use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default MeetoOps HTTP/WebSocket listen port (chosen to avoid common collisions like 8080/3000/9000).
pub const DEFAULT_LISTEN_PORT: u16 = 9480;

/// Application initialization state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SetupStage {
    Uninitialized,
    FirstRunSetup,
    AdminCreated,
    InfrastructureSetup,
    Ready,
}

impl SetupStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uninitialized => "UNINITIALIZED",
            Self::FirstRunSetup => "FIRST_RUN_SETUP",
            Self::AdminCreated => "ADMIN_CREATED",
            Self::InfrastructureSetup => "INFRASTRUCTURE_SETUP",
            Self::Ready => "READY",
        }
    }

    pub fn from_db_str(value: &str) -> Result<Self, CommonError> {
        match value {
            "UNINITIALIZED" => Ok(Self::Uninitialized),
            "FIRST_RUN_SETUP" => Ok(Self::FirstRunSetup),
            "ADMIN_CREATED" => Ok(Self::AdminCreated),
            "INFRASTRUCTURE_SETUP" => Ok(Self::InfrastructureSetup),
            "READY" => Ok(Self::Ready),
            other => Err(CommonError::InvalidState(format!(
                "unknown setup stage: {other}"
            ))),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Built-in RBAC roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SuperAdmin,
    Admin,
    Operator,
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SuperAdmin => "super_admin",
            Self::Admin => "admin",
            Self::Operator => "operator",
            Self::Viewer => "viewer",
        }
    }

    pub fn from_db_str(value: &str) -> Result<Self, CommonError> {
        match value {
            "super_admin" => Ok(Self::SuperAdmin),
            "admin" => Ok(Self::Admin),
            "operator" => Ok(Self::Operator),
            "viewer" => Ok(Self::Viewer),
            other => Err(CommonError::Validation(format!("unknown role: {other}"))),
        }
    }
}

/// Standard API success envelope.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn paginated(data: T, meta: PaginationMeta) -> Self {
        Self {
            data,
            meta: Some(meta),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

impl PaginationMeta {
    pub fn new(page: u32, per_page: u32, total: u64) -> Self {
        let safe_per_page = per_page.clamp(1, 200);
        let total_pages = if total == 0 {
            1
        } else {
            ((total + safe_per_page as u64 - 1) / safe_per_page as u64) as u32
        };
        Self {
            page: page.max(1),
            per_page: safe_per_page,
            total,
            total_pages,
        }
    }
}

/// Standard API error envelope.
#[derive(Debug, Serialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: String,
}

#[derive(Debug, Error)]
pub enum CommonError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
}

/// Keywords that indicate a container environment variable contains sensitive secret material.
const SECRET_ENV_PATTERNS: &[&str] = &[
    "PASSWORD",
    "PASSWD",
    "SECRET",
    "TOKEN",
    "API_KEY",
    "APIKEY",
    "ACCESS_KEY",
    "PRIVATE_KEY",
    "PRIVATE",
    "CREDENTIAL",
    "AUTH",
    "DSN",
    "CONNECTION_STRING",
    "DATABASE_URL",
    "REDIS_URL",
    "MONGO_URI",
    "CERT",
];

/// Returns true if an environment variable name is considered sensitive and must be masked by default.
pub fn is_secret_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    SECRET_ENV_PATTERNS.iter().any(|pat| upper.contains(pat))
}

/// Masks a secret value if the key matches sensitive patterns.
pub fn mask_env_pair(key: &str, value: &str) -> (String, bool) {
    if is_secret_env_key(key) {
        ("••••••••".to_string(), true)
    } else {
        (value.to_string(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_sensitive_env_keys() {
        assert!(is_secret_env_key("POSTGRES_PASSWORD"));
        assert!(is_secret_env_key("jwt_secret"));
        assert!(is_secret_env_key("STRIPE_API_KEY"));
        assert!(is_secret_env_key("DATABASE_URL"));
        assert!(!is_secret_env_key("NODE_ENV"));
        assert!(!is_secret_env_key("TZ"));

        let (masked, is_secret) = mask_env_pair("MYSQL_ROOT_PASSWORD", "s3cr3t");
        assert!(is_secret);
        assert_eq!(masked, "••••••••");

        let (plain, is_secret) = mask_env_pair("APP_PORT", "9480");
        assert!(!is_secret);
        assert_eq!(plain, "9480");
    }
}
