use futures_util::TryStreamExt;
use mongodb::bson::{doc, Document};
use serde::{Deserialize, Serialize};
use sqlx::{
    mysql::MySqlPoolOptions, postgres::PgPoolOptions, Column, Row, TypeInfo, ValueRef,
};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseExplorerError {
    #[error("unsupported database engine: {0}")]
    UnsupportedEngine(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("read-only mode active: mutating queries are blocked")]
    ReadOnlyBlocked,
    #[error("sql driver error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("mongodb driver error: {0}")]
    Mongo(#[from] mongodb::error::Error),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DbConnectionConfig {
    pub engine: String, // "postgres" | "mysql" | "mariadb" | "mongodb"
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
}

impl std::fmt::Debug for DbConnectionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnectionConfig")
            .field("engine", &self.engine)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("database", &self.database)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfoDto {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResultDto {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub truncated: bool,
}

fn validate_identifier(name: &str) -> Result<&str, DatabaseExplorerError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.len() > 128
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(DatabaseExplorerError::Validation(format!(
            "Invalid SQL/collection identifier: '{name}'"
        )));
    }
    Ok(trimmed)
}

fn build_pg_url(cfg: &DbConnectionConfig, db_override: Option<&str>) -> String {
    let user = urlencoding::encode(&cfg.username);
    let pass = urlencoding::encode(&cfg.password);
    let db = db_override
        .or(cfg.database.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or("postgres");
    format!(
        "postgres://{user}:{pass}@{}:{}/{}",
        cfg.host,
        cfg.port,
        urlencoding::encode(db)
    )
}

fn build_mysql_url(cfg: &DbConnectionConfig, db_override: Option<&str>) -> String {
    let user = urlencoding::encode(&cfg.username);
    let pass = urlencoding::encode(&cfg.password);
    let db = db_override
        .or(cfg.database.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    format!(
        "mysql://{user}:{pass}@{}:{}/{}",
        cfg.host,
        cfg.port,
        urlencoding::encode(db)
    )
}

fn build_mongo_url(cfg: &DbConnectionConfig) -> String {
    if cfg.username.is_empty() {
        format!("mongodb://{}:{}/", cfg.host, cfg.port)
    } else {
        let user = urlencoding::encode(&cfg.username);
        let pass = urlencoding::encode(&cfg.password);
        format!("mongodb://{user}:{pass}@{}:{}/", cfg.host, cfg.port)
    }
}

/// Tests real database authentication using the official driver (no shell commands).
pub async fn test_connection(cfg: &DbConnectionConfig) -> Result<String, DatabaseExplorerError> {
    match cfg.engine.as_str() {
        "postgres" => {
            let url = build_pg_url(cfg, None);
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&url)
                .await?;
            let (ver,): (String,) = sqlx::query_as("SELECT version()").fetch_one(&pool).await?;
            pool.close().await;
            Ok(ver)
        }
        "mysql" | "mariadb" => {
            let url = build_mysql_url(cfg, None);
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&url)
                .await?;
            let (ver,): (String,) = sqlx::query_as("SELECT VERSION()").fetch_one(&pool).await?;
            pool.close().await;
            Ok(ver)
        }
        "mongodb" => {
            let client = mongodb::Client::with_uri_str(build_mongo_url(cfg)).await?;
            let resp = client
                .database("admin")
                .run_command(doc! { "ping": 1 })
                .await?;
            Ok(format!("MongoDB ping ok: {resp}"))
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

pub async fn list_databases(cfg: &DbConnectionConfig) -> Result<Vec<String>, DatabaseExplorerError> {
    match cfg.engine.as_str() {
        "postgres" => {
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_pg_url(cfg, None))
                .await?;
            let rows = sqlx::query(
                "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            )
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            Ok(rows.into_iter().filter_map(|r| r.try_get::<String, _>(0).ok()).collect())
        }
        "mysql" | "mariadb" => {
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_mysql_url(cfg, None))
                .await?;
            let rows = sqlx::query("SHOW DATABASES").fetch_all(&pool).await?;
            pool.close().await;
            Ok(rows.into_iter().filter_map(|r| r.try_get::<String, _>(0).ok()).collect())
        }
        "mongodb" => {
            let client = mongodb::Client::with_uri_str(build_mongo_url(cfg)).await?;
            Ok(client.list_database_names().await?)
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

pub async fn list_tables(
    cfg: &DbConnectionConfig,
    database: &str,
) -> Result<Vec<String>, DatabaseExplorerError> {
    let db = validate_identifier(database)?;
    match cfg.engine.as_str() {
        "postgres" => {
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_pg_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = 'public' ORDER BY table_name",
            )
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            Ok(rows.into_iter().filter_map(|r| r.try_get::<String, _>(0).ok()).collect())
        }
        "mysql" | "mariadb" => {
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_mysql_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = ? ORDER BY table_name",
            )
            .bind(db)
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            Ok(rows.into_iter().filter_map(|r| r.try_get::<String, _>(0).ok()).collect())
        }
        "mongodb" => {
            let client = mongodb::Client::with_uri_str(build_mongo_url(cfg)).await?;
            Ok(client.database(db).list_collection_names().await?)
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

pub async fn inspect_columns(
    cfg: &DbConnectionConfig,
    database: &str,
    table: &str,
) -> Result<Vec<ColumnInfoDto>, DatabaseExplorerError> {
    let db = validate_identifier(database)?;
    let tbl = validate_identifier(table)?;
    match cfg.engine.as_str() {
        "postgres" => {
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_pg_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(
                "SELECT column_name, data_type, is_nullable, column_default
                 FROM information_schema.columns
                 WHERE table_schema = 'public' AND table_name = $1
                 ORDER BY ordinal_position",
            )
            .bind(tbl)
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            Ok(rows
                .into_iter()
                .map(|r| ColumnInfoDto {
                    name: r.try_get("column_name").unwrap_or_default(),
                    data_type: r.try_get("data_type").unwrap_or_default(),
                    is_nullable: r
                        .try_get::<String, _>("is_nullable")
                        .map(|v| v.eq_ignore_ascii_case("YES"))
                        .unwrap_or(true),
                    default_value: r.try_get("column_default").ok(),
                })
                .collect())
        }
        "mysql" | "mariadb" => {
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_mysql_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(
                "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT
                 FROM information_schema.columns
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
                 ORDER BY ORDINAL_POSITION",
            )
            .bind(db)
            .bind(tbl)
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            Ok(rows
                .into_iter()
                .map(|r| ColumnInfoDto {
                    name: r.try_get(0).unwrap_or_default(),
                    data_type: r.try_get(1).unwrap_or_default(),
                    is_nullable: r
                        .try_get::<String, _>(2)
                        .map(|v| v.eq_ignore_ascii_case("YES"))
                        .unwrap_or(true),
                    default_value: r.try_get(3).ok(),
                })
                .collect())
        }
        "mongodb" => {
            let client = mongodb::Client::with_uri_str(build_mongo_url(cfg)).await?;
            let coll = client.database(db).collection::<Document>(tbl);
            let sample = coll.find_one(doc! {}).await?;
            let cols = sample
                .map(|d| {
                    d.keys()
                        .map(|k| ColumnInfoDto {
                            name: k.clone(),
                            data_type: "bson".to_string(),
                            is_nullable: true,
                            default_value: None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(cols)
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

pub async fn browse_rows(
    cfg: &DbConnectionConfig,
    database: &str,
    table: &str,
    limit: u32,
    offset: u32,
) -> Result<QueryResultDto, DatabaseExplorerError> {
    let db = validate_identifier(database)?;
    let tbl = validate_identifier(table)?;
    let safe_limit = limit.clamp(1, 200) as i64;
    let safe_offset = offset as i64;

    match cfg.engine.as_str() {
        "postgres" => {
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_pg_url(cfg, Some(db)))
                .await?;
            let sql = format!("SELECT * FROM \"{tbl}\" LIMIT $1 OFFSET $2");
            let rows = sqlx::query(&sql)
                .bind(safe_limit)
                .bind(safe_offset)
                .fetch_all(&pool)
                .await?;
            pool.close().await;
            Ok(format_pg_rows(&rows, safe_limit as usize))
        }
        "mysql" | "mariadb" => {
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_mysql_url(cfg, Some(db)))
                .await?;
            let sql = format!("SELECT * FROM `{tbl}` LIMIT ? OFFSET ?");
            let rows = sqlx::query(&sql)
                .bind(safe_limit)
                .bind(safe_offset)
                .fetch_all(&pool)
                .await?;
            pool.close().await;
            Ok(format_mysql_rows(&rows, safe_limit as usize))
        }
        "mongodb" => {
            let client = mongodb::Client::with_uri_str(build_mongo_url(cfg)).await?;
            let coll = client.database(db).collection::<Document>(tbl);
            let mut cursor = coll
                .find(doc! {})
                .skip(safe_offset as u64)
                .limit(safe_limit)
                .await?;
            let mut out_rows = Vec::new();
            while let Some(doc) = cursor.try_next().await? {
                let val = serde_json::to_value(doc).unwrap_or(serde_json::Value::Null);
                out_rows.push(vec![val]);
            }
            let row_count = out_rows.len();
            Ok(QueryResultDto {
                columns: vec!["document".to_string()],
                rows: out_rows,
                row_count,
                truncated: row_count >= safe_limit as usize,
            })
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

/// Checks if a SQL string contains mutating keywords when read-only mode is enforced.
pub fn is_mutating_sql(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    let forbidden = [
        "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE", "GRANT", "REVOKE",
        "REPLACE", "MERGE", "EXEC", "CALL",
    ];
    forbidden
        .iter()
        .any(|kw| upper.starts_with(kw) || upper.contains(&format!(" {kw} ")) || upper.contains(&format!(";{kw}")))
}

pub async fn execute_sql_query(
    cfg: &DbConnectionConfig,
    database: &str,
    sql: &str,
    read_only: bool,
) -> Result<QueryResultDto, DatabaseExplorerError> {
    let db = validate_identifier(database)?;
    if sql.trim().is_empty() {
        return Err(DatabaseExplorerError::Validation("Query cannot be empty.".to_string()));
    }
    if read_only && is_mutating_sql(sql) {
        return Err(DatabaseExplorerError::ReadOnlyBlocked);
    }

    match cfg.engine.as_str() {
        "postgres" => {
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_pg_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(sql).fetch_all(&pool).await?;
            pool.close().await;
            Ok(format_pg_rows(&rows, 500))
        }
        "mysql" | "mariadb" => {
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(6))
                .connect(&build_mysql_url(cfg, Some(db)))
                .await?;
            let rows = sqlx::query(sql).fetch_all(&pool).await?;
            pool.close().await;
            Ok(format_mysql_rows(&rows, 500))
        }
        other => Err(DatabaseExplorerError::UnsupportedEngine(other.to_string())),
    }
}

fn format_pg_rows(rows: &[sqlx::postgres::PgRow], max_rows: usize) -> QueryResultDto {
    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();
    let truncated = rows.len() > max_rows;
    let out_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .take(max_rows)
        .map(|row| {
            (0..columns.len())
                .map(|i| pg_cell_to_json(row, i))
                .collect()
        })
        .collect();
    let row_count = out_rows.len();
    QueryResultDto {
        columns,
        rows: out_rows,
        row_count,
        truncated,
    }
}

fn pg_cell_to_json(row: &sqlx::postgres::PgRow, idx: usize) -> serde_json::Value {
    if let Ok(raw) = row.try_get_raw(idx) {
        if raw.is_null() {
            return serde_json::Value::Null;
        }
    }
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<i32, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(idx) {
        return v;
    }
    let ty = row.columns()[idx].type_info().name();
    serde_json::Value::String(format!("<{ty}>"))
}

fn format_mysql_rows(rows: &[sqlx::mysql::MySqlRow], max_rows: usize) -> QueryResultDto {
    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();
    let truncated = rows.len() > max_rows;
    let out_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .take(max_rows)
        .map(|row| {
            (0..columns.len())
                .map(|i| mysql_cell_to_json(row, i))
                .collect()
        })
        .collect();
    let row_count = out_rows.len();
    QueryResultDto {
        columns,
        rows: out_rows,
        row_count,
        truncated,
    }
}

fn mysql_cell_to_json(row: &sqlx::mysql::MySqlRow, idx: usize) -> serde_json::Value {
    if let Ok(raw) = row.try_get_raw(idx) {
        if raw.is_null() {
            return serde_json::Value::Null;
        }
    }
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<u64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(idx) {
        return serde_json::json!(v);
    }
    let ty = row.columns()[idx].type_info().name();
    serde_json::Value::String(format!("<{ty}>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guards_mutating_sql_in_read_only_mode() {
        assert!(!is_mutating_sql("SELECT * FROM users LIMIT 10"));
        assert!(is_mutating_sql("DELETE FROM users WHERE id = 1"));
        assert!(is_mutating_sql("DROP TABLE audit_logs"));
        assert!(is_mutating_sql("SELECT 1; UPDATE users SET role='admin'"));
    }
}
