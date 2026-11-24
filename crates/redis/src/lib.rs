use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedisExplorerError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("destructive command '{0}' requires explicit confirmation and redis:admin permission")]
    DestructiveCommandBlocked(String),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RedisConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: Option<i64>,
}

impl std::fmt::Debug for RedisConnectionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisConnectionConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("db", &self.db)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RedisOverviewDto {
    pub version: Option<String>,
    pub used_memory_human: Option<String>,
    pub used_memory_bytes: Option<u64>,
    pub total_keys: u64,
    pub connected_clients: Option<u64>,
    pub ops_per_sec: Option<u64>,
    pub keyspace_hits: Option<u64>,
    pub keyspace_misses: Option<u64>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedisKeyItemDto {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedisScanPageDto {
    pub next_cursor: u64,
    pub keys: Vec<RedisKeyItemDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedisKeyDetailDto {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub value: serde_json::Value,
}

fn build_redis_url(cfg: &RedisConnectionConfig) -> String {
    let db = cfg.db.unwrap_or(0);
    let auth = match (
        cfg.username.as_deref().filter(|s| !s.is_empty()),
        cfg.password.as_deref().filter(|s| !s.is_empty()),
    ) {
        (Some(u), Some(p)) => format!("{}:{}@", urlencoding::encode(u), urlencoding::encode(p)),
        (None, Some(p)) => format!(":{}@", urlencoding::encode(p)),
        _ => String::new(),
    };
    format!("redis://{auth}{}:{}/{db}", cfg.host, cfg.port)
}

async fn connect(cfg: &RedisConnectionConfig) -> Result<redis::aio::MultiplexedConnection, RedisExplorerError> {
    let client = redis::Client::open(build_redis_url(cfg))?;
    Ok(client.get_multiplexed_async_connection().await?)
}

pub async fn test_connection(cfg: &RedisConnectionConfig) -> Result<String, RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(pong)
}

pub async fn get_overview(cfg: &RedisConnectionConfig) -> Result<RedisOverviewDto, RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    let info_raw: String = redis::cmd("INFO").query_async(&mut conn).await?;
    let total_keys: u64 = redis::cmd("DBSIZE").query_async(&mut conn).await?;

    let mut map = HashMap::new();
    for line in info_raw.lines() {
        if let Some((k, v)) = line.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    Ok(RedisOverviewDto {
        version: map.get("redis_version").cloned(),
        used_memory_human: map.get("used_memory_human").cloned(),
        used_memory_bytes: map.get("used_memory").and_then(|v| v.parse().ok()),
        total_keys,
        connected_clients: map.get("connected_clients").and_then(|v| v.parse().ok()),
        ops_per_sec: map
            .get("instantaneous_ops_per_sec")
            .and_then(|v| v.parse().ok()),
        keyspace_hits: map.get("keyspace_hits").and_then(|v| v.parse().ok()),
        keyspace_misses: map.get("keyspace_misses").and_then(|v| v.parse().ok()),
        uptime_seconds: map.get("uptime_in_seconds").and_then(|v| v.parse().ok()),
    })
}

/// Cursor-based key browsing using `SCAN` (NEVER `KEYS *`).
pub async fn scan_keys(
    cfg: &RedisConnectionConfig,
    cursor: u64,
    pattern: &str,
    count: u32,
) -> Result<RedisScanPageDto, RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    let safe_pattern = if pattern.trim().is_empty() {
        "*"
    } else {
        pattern.trim()
    };
    let safe_count = count.clamp(10, 200);

    let (next_cursor, raw_keys): (u64, Vec<String>) = redis::cmd("SCAN")
        .arg(cursor)
        .arg("MATCH")
        .arg(safe_pattern)
        .arg("COUNT")
        .arg(safe_count)
        .query_async(&mut conn)
        .await?;

    let mut items = Vec::with_capacity(raw_keys.len());
    for key in raw_keys {
        let key_type: String = redis::cmd("TYPE")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(-2);
        items.push(RedisKeyItemDto { key, key_type, ttl });
    }

    Ok(RedisScanPageDto {
        next_cursor,
        keys: items,
    })
}

pub async fn get_key_detail(
    cfg: &RedisConnectionConfig,
    key: &str,
) -> Result<RedisKeyDetailDto, RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    let key_type: String = redis::cmd("TYPE").arg(key).query_async(&mut conn).await?;
    let ttl: i64 = redis::cmd("TTL").arg(key).query_async(&mut conn).await?;

    let value = match key_type.as_str() {
        "string" => {
            let v: Option<String> = conn.get(key).await?;
            serde_json::json!(v)
        }
        "hash" => {
            let v: HashMap<String, String> = conn.hgetall(key).await?;
            serde_json::json!(v)
        }
        "list" => {
            let v: Vec<String> = conn.lrange(key, 0, 199).await?;
            serde_json::json!(v)
        }
        "set" => {
            let v: Vec<String> = conn.smembers(key).await?;
            serde_json::json!(v)
        }
        "zset" => {
            let v: Vec<(String, f64)> = conn.zrange_withscores(key, 0, 199).await?;
            serde_json::json!(v)
        }
        _ => serde_json::json!(format!("<type: {key_type}>")),
    };

    Ok(RedisKeyDetailDto {
        key: key.to_string(),
        key_type,
        ttl,
        value,
    })
}

pub async fn set_string_key(
    cfg: &RedisConnectionConfig,
    key: &str,
    value: &str,
    ttl_secs: Option<u64>,
) -> Result<(), RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    if let Some(ttl) = ttl_secs.filter(|t| *t > 0) {
        let _: () = conn.set_ex(key, value, ttl).await?;
    } else {
        let _: () = conn.set(key, value).await?;
    }
    Ok(())
}

pub async fn delete_key(cfg: &RedisConnectionConfig, key: &str) -> Result<u64, RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    let deleted: u64 = conn.del(key).await?;
    Ok(deleted)
}

pub async fn rename_key(
    cfg: &RedisConnectionConfig,
    old_key: &str,
    new_key: &str,
) -> Result<(), RedisExplorerError> {
    if old_key.trim().is_empty() || new_key.trim().is_empty() {
        return Err(RedisExplorerError::Validation(
            "Both old_key and new_key are required.".to_string(),
        ));
    }
    let mut conn = connect(cfg).await?;
    let _: () = conn.rename(old_key, new_key).await?;
    Ok(())
}

pub async fn mutate_collection_key(
    cfg: &RedisConnectionConfig,
    key: &str,
    key_type: &str,
    field: Option<&str>,
    value: &str,
    score: Option<f64>,
) -> Result<(), RedisExplorerError> {
    let mut conn = connect(cfg).await?;
    match key_type {
        "hash" => {
            let f = field
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| RedisExplorerError::Validation("Hash field is required.".to_string()))?;
            let _: () = conn.hset(key, f, value).await?;
        }
        "list" => {
            let _: () = conn.rpush(key, value).await?;
        }
        "set" => {
            let _: () = conn.sadd(key, value).await?;
        }
        "zset" => {
            let sc = score.unwrap_or(0.0);
            let _: () = conn.zadd(key, value, sc).await?;
        }
        _ => {
            let _: () = conn.set(key, value).await?;
        }
    }
    Ok(())
}

const DESTRUCTIVE_COMMANDS: &[&str] = &[
    "FLUSHALL",
    "FLUSHDB",
    "SHUTDOWN",
    "CONFIG",
    "DEBUG",
    "SLAVEOF",
    "REPLICAOF",
    "MODULE",
    "ACL",
    "DEL",
    "UNLINK",
];

pub fn is_destructive_redis_command(cmd_name: &str) -> bool {
    let upper = cmd_name.trim().to_ascii_uppercase();
    DESTRUCTIVE_COMMANDS.iter().any(|c| *c == upper)
}

pub async fn execute_command(
    cfg: &RedisConnectionConfig,
    command_line: &str,
    allow_destructive: bool,
) -> Result<String, RedisExplorerError> {
    let parts: Vec<&str> = command_line.split_whitespace().collect();
    if parts.is_empty() {
        return Err(RedisExplorerError::Validation(
            "Command cannot be empty.".to_string(),
        ));
    }
    let cmd_name = parts[0];
    if is_destructive_redis_command(cmd_name) && !allow_destructive {
        return Err(RedisExplorerError::DestructiveCommandBlocked(
            cmd_name.to_ascii_uppercase(),
        ));
    }

    let mut conn = connect(cfg).await?;
    let mut cmd = redis::cmd(cmd_name);
    for arg in &parts[1..] {
        cmd.arg(*arg);
    }
    let val: redis::Value = cmd.query_async(&mut conn).await?;
    Ok(format!("{val:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_destructive_redis_commands() {
        assert!(is_destructive_redis_command("FLUSHALL"));
        assert!(is_destructive_redis_command("flushdb"));
        assert!(is_destructive_redis_command("CONFIG"));
        assert!(!is_destructive_redis_command("GET"));
        assert!(!is_destructive_redis_command("SCAN"));
    }
}
