use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use meetoops_common::{ApiErrorBody, ApiErrorEnvelope};
use meetoops_core::CoreError;
use uuid::Uuid;

pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg)
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", msg)
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg)
    }

    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", msg)
    }
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Validation(msg) => Self::bad_request(msg),
            CoreError::Password(e) => Self::bad_request(e.to_string()),
            CoreError::SetupLocked => Self::new(
                StatusCode::FORBIDDEN,
                "SETUP_LOCKED",
                "Initial setup has already been completed and is permanently locked.",
            ),
            CoreError::Unauthorized(msg) => Self::unauthorized(msg),
            CoreError::Forbidden(msg) => Self::forbidden(msg),
            CoreError::NotFound(msg) => Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            CoreError::Conflict(msg) => Self::new(StatusCode::CONFLICT, "CONFLICT", msg),
            CoreError::Database(e) => {
                tracing::error!(error = %e, "database error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal database error occurred.",
                )
            }
            CoreError::Migration(e) => {
                tracing::error!(error = %e, "migration error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Database migration error.",
                )
            }
            CoreError::Vault(e) => {
                tracing::error!(error = %e, "vault error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "VAULT_ERROR",
                    "Encryption vault error.",
                )
            }
            CoreError::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected internal error occurred.",
                )
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let envelope = ApiErrorEnvelope {
            error: ApiErrorBody {
                code: self.code.to_string(),
                message: self.message,
                request_id: format!("req_{}", Uuid::now_v7()),
            },
        };
        (self.status, Json(envelope)).into_response()
    }
}
