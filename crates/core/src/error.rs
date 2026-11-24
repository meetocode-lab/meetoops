use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("security vault error: {0}")]
    Vault(#[from] meetoops_security::VaultError),
    #[error("password error: {0}")]
    Password(#[from] meetoops_security::PasswordError),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("setup is already completed and permanently locked")]
    SetupLocked,
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}
