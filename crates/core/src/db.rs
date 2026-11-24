use crate::error::CoreError;
use chrono::Utc;
use meetoops_common::SetupStage;
use meetoops_security::MasterKeyVault;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Initializes SQLite in WAL mode, runs forward-only migrations automatically,
/// and verifies or records the master encryption key fingerprint in `app_state`.
pub async fn init_database(
    db_path: &Path,
    vault: &MasterKeyVault,
) -> Result<SqlitePool, CoreError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CoreError::Internal(format!("failed to create db directory: {e}")))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    ensure_app_state_and_verify_key(&pool, vault.fingerprint()).await?;

    Ok(pool)
}

/// Initializes an in-memory SQLite pool for fast isolated unit/integration tests.
pub async fn init_in_memory_database(vault: &MasterKeyVault) -> Result<SqlitePool, CoreError> {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;
    ensure_app_state_and_verify_key(&pool, vault.fingerprint()).await?;
    Ok(pool)
}

async fn ensure_app_state_and_verify_key(
    pool: &SqlitePool,
    key_fingerprint: &str,
) -> Result<(), CoreError> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT key_fingerprint FROM app_state WHERE id = 1")
            .fetch_optional(pool)
            .await?;

    match existing {
        None => {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO app_state (id, setup_stage, initialized_at, schema_version, key_fingerprint, updated_at)
                 VALUES (1, ?, NULL, 1, ?, ?)",
            )
            .bind(SetupStage::FirstRunSetup.as_str())
            .bind(key_fingerprint)
            .bind(&now)
            .execute(pool)
            .await?;
            Ok(())
        }
        Some((stored_fp,)) => {
            if stored_fp != key_fingerprint {
                return Err(CoreError::Vault(meetoops_security::VaultError::KeyMismatch));
            }
            Ok(())
        }
    }
}
