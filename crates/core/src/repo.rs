use crate::error::CoreError;
use crate::models::{
    AlertChannelDto, AlertEventDto, AlertRuleDto, ApiTokenDto, AppSettingsDto, AppStateRecord,
    AuditLogEntry, AuthenticatedSession, DiscoveredServiceDto, HostMetricPoint,
    ProjectDeploymentDto, ProjectDto, ServerSummary, SshHostDto, UserPublic,
};
use chrono::{Duration, Utc};
use meetoops_common::{Role, SetupStage};
use meetoops_security::{
    generate_csrf_token, generate_opaque_token, hash_password, hash_token,
    validate_password_strength, verify_password, MasterKeyVault,
};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct CoreStore {
    pool: SqlitePool,
}

impl CoreStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn get_app_state(&self) -> Result<AppStateRecord, CoreError> {
        let row = sqlx::query(
            "SELECT setup_stage, initialized_at, key_fingerprint FROM app_state WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await?;

        let stage_str: String = row.try_get("setup_stage")?;
        let initialized_at: Option<String> = row.try_get("initialized_at")?;
        let fp: String = row.try_get("key_fingerprint")?;

        let setup_stage = SetupStage::from_db_str(&stage_str)
            .map_err(|e| CoreError::Internal(e.to_string()))?;

        let short = fp.chars().take(12).collect::<String>();
        Ok(AppStateRecord {
            setup_stage,
            initialized_at,
            key_fingerprint_short: short,
        })
    }

    /// Atomically creates the initial Super Administrator and advances `setup_stage` to `ADMIN_CREATED`.
    /// Fails with `CoreError::SetupLocked` if an admin already exists or setup is already past `FIRST_RUN_SETUP`.
    pub async fn create_initial_super_admin(
        &self,
        full_name: &str,
        email: &str,
        username: Option<&str>,
        password: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(UserPublic, String, String), CoreError> {
        let full_name = full_name.trim();
        let email = email.trim().to_ascii_lowercase();
        let username = username
            .map(|u| u.trim().to_ascii_lowercase())
            .filter(|u| !u.is_empty());

        if full_name.len() < 2 {
            return Err(CoreError::Validation(
                "Full name must be at least 2 characters.".to_string(),
            ));
        }
        if !email.contains('@') || email.len() < 5 {
            return Err(CoreError::Validation(
                "Please provide a valid email address.".to_string(),
            ));
        }
        validate_password_strength(password)?;

        // Hash before opening write transaction so we don't hold SQLite write lock during Argon2id
        let password_hash = hash_password(password.to_string()).await?;
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let user_id = Uuid::now_v7().to_string();

        let session_token = generate_opaque_token();
        let session_token_hash = hash_token(&session_token);
        let csrf_token = generate_csrf_token();
        let session_id = Uuid::now_v7().to_string();
        let expires_at = (now + Duration::hours(24)).to_rfc3339();

        let mut tx = self.pool.begin().await?;

        let (stage_str,): (String,) =
            sqlx::query_as("SELECT setup_stage FROM app_state WHERE id = 1")
                .fetch_one(&mut *tx)
                .await?;
        let (user_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&mut *tx)
            .await?;

        if stage_str != SetupStage::FirstRunSetup.as_str() || user_count > 0 {
            tx.rollback().await?;
            return Err(CoreError::SetupLocked);
        }

        sqlx::query(
            "INSERT INTO users (id, full_name, email, username, password_hash, role, is_active, created_at, updated_at, last_login_at)
             VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(&user_id)
        .bind(full_name)
        .bind(&email)
        .bind(&username)
        .bind(&password_hash)
        .bind(Role::SuperAdmin.as_str())
        .bind(&now_str)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE app_state SET setup_stage = ?, updated_at = ? WHERE id = 1")
            .bind(SetupStage::AdminCreated.as_str())
            .bind(&now_str)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, csrf_token, ip_address, user_agent, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session_id)
        .bind(&user_id)
        .bind(&session_token_hash)
        .bind(&csrf_token)
        .bind(ip_address)
        .bind(user_agent)
        .bind(&expires_at)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.write_audit_log(
            Some(&user_id),
            Some(&email),
            "setup.admin_created",
            "user",
            Some(&user_id),
            None,
            "success",
            ip_address,
            serde_json::json!({ "role": "super_admin" }),
        )
        .await?;

        let user = UserPublic {
            id: user_id,
            full_name: full_name.to_string(),
            email,
            username,
            role: Role::SuperAdmin,
            is_active: true,
            last_login_at: Some(now_str.clone()),
            created_at: now_str,
        };

        Ok((user, session_token, csrf_token))
    }

    pub async fn set_setup_stage(&self, stage: SetupStage) -> Result<(), CoreError> {
        let current = self.get_app_state().await?;
        if current.setup_stage.is_ready() {
            return Err(CoreError::SetupLocked);
        }
        let now = Utc::now().to_rfc3339();
        if stage.is_ready() {
            sqlx::query(
                "UPDATE app_state SET setup_stage = ?, initialized_at = ?, updated_at = ? WHERE id = 1",
            )
            .bind(stage.as_str())
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("UPDATE app_state SET setup_stage = ?, updated_at = ? WHERE id = 1")
                .bind(stage.as_str())
                .bind(&now)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn authenticate_user(
        &self,
        identifier: &str,
        password: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(UserPublic, String, String), CoreError> {
        let ident = identifier.trim().to_ascii_lowercase();
        let row = sqlx::query(
            "SELECT id, full_name, email, username, password_hash, role, is_active,
                    failed_login_attempts, locked_until, created_at
             FROM users
             WHERE email = ? OR username = ?
             LIMIT 1",
        )
        .bind(&ident)
        .bind(&ident)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            self.write_audit_log(
                None,
                Some(&ident),
                "auth.login_failed",
                "session",
                None,
                None,
                "denied",
                ip_address,
                serde_json::json!({ "reason": "invalid_credentials" }),
            )
            .await?;
            return Err(CoreError::Unauthorized(
                "Invalid email/username or password.".to_string(),
            ));
        };

        let user_id: String = row.try_get("id")?;
        let email: String = row.try_get("email")?;
        let is_active: i64 = row.try_get("is_active")?;
        let locked_until: Option<String> = row.try_get("locked_until")?;
        let failed_attempts: i64 = row.try_get("failed_login_attempts")?;

        if is_active == 0 {
            return Err(CoreError::Forbidden(
                "This account has been disabled.".to_string(),
            ));
        }

        if let Some(ref until_str) = locked_until {
            if let Ok(until) = chrono::DateTime::parse_from_rfc3339(until_str) {
                if until.with_timezone(&Utc) > Utc::now() {
                    return Err(CoreError::Unauthorized(
                        "Account temporarily locked due to too many failed attempts.".to_string(),
                    ));
                }
            }
        }

        let password_hash: String = row.try_get("password_hash")?;
        let valid = verify_password(password.to_string(), password_hash).await?;

        let settings = self.get_settings().await.unwrap_or_default();
        if !valid {
            let next_attempts = failed_attempts + 1;
            let lock_ts = if next_attempts >= settings.max_login_attempts as i64 {
                Some((Utc::now() + Duration::minutes(15)).to_rfc3339())
            } else {
                None
            };
            sqlx::query(
                "UPDATE users SET failed_login_attempts = ?, locked_until = ?, updated_at = ? WHERE id = ?",
            )
            .bind(next_attempts)
            .bind(&lock_ts)
            .bind(Utc::now().to_rfc3339())
            .bind(&user_id)
            .execute(&self.pool)
            .await?;

            self.write_audit_log(
                Some(&user_id),
                Some(&email),
                "auth.login_failed",
                "session",
                None,
                None,
                "denied",
                ip_address,
                serde_json::json!({ "failed_attempts": next_attempts }),
            )
            .await?;
            return Err(CoreError::Unauthorized(
                "Invalid email/username or password.".to_string(),
            ));
        }

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let session_token = generate_opaque_token();
        let session_token_hash = hash_token(&session_token);
        let csrf_token = generate_csrf_token();
        let session_id = Uuid::now_v7().to_string();
        let expires_at =
            (now + Duration::hours(settings.session_timeout_hours.max(1) as i64)).to_rfc3339();

        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, last_login_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&now_str)
        .bind(&now_str)
        .bind(&user_id)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, csrf_token, ip_address, user_agent, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session_id)
        .bind(&user_id)
        .bind(&session_token_hash)
        .bind(&csrf_token)
        .bind(ip_address)
        .bind(user_agent)
        .bind(&expires_at)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            Some(&user_id),
            Some(&email),
            "auth.login",
            "session",
            Some(&session_id),
            None,
            "success",
            ip_address,
            serde_json::json!({}),
        )
        .await?;

        let role_str: String = row.try_get("role")?;
        let role = Role::from_db_str(&role_str).map_err(|e| CoreError::Internal(e.to_string()))?;

        let user = UserPublic {
            id: user_id,
            full_name: row.try_get("full_name")?,
            email,
            username: row.try_get("username")?,
            role,
            is_active: true,
            last_login_at: Some(now_str),
            created_at: row.try_get("created_at")?,
        };

        Ok((user, session_token, csrf_token))
    }

    pub async fn resolve_session(
        &self,
        raw_token: &str,
    ) -> Result<Option<AuthenticatedSession>, CoreError> {
        let token_hash = hash_token(raw_token);
        let row = sqlx::query(
            "SELECT s.id AS session_id, s.csrf_token, s.expires_at, s.revoked_at,
                    u.id AS user_id, u.full_name, u.email, u.username, u.role, u.is_active,
                    u.last_login_at, u.created_at
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = ?
             LIMIT 1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let revoked: Option<String> = row.try_get("revoked_at")?;
        if revoked.is_some() {
            return Ok(None);
        }

        let is_active: i64 = row.try_get("is_active")?;
        if is_active == 0 {
            return Ok(None);
        }

        let expires_str: String = row.try_get("expires_at")?;
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&expires_str) {
            if exp.with_timezone(&Utc) <= Utc::now() {
                return Ok(None);
            }
        }

        let role_str: String = row.try_get("role")?;
        let role = Role::from_db_str(&role_str).map_err(|e| CoreError::Internal(e.to_string()))?;

        Ok(Some(AuthenticatedSession {
            session_id: row.try_get("session_id")?,
            csrf_token: row.try_get("csrf_token")?,
            user: UserPublic {
                id: row.try_get("user_id")?,
                full_name: row.try_get("full_name")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                role,
                is_active: true,
                last_login_at: row.try_get("last_login_at")?,
                created_at: row.try_get("created_at")?,
            },
        }))
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), CoreError> {
        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<UserPublic>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, full_name, email, username, role, is_active, last_login_at, created_at
             FROM users ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let role_str: String = row.try_get("role")?;
            let role =
                Role::from_db_str(&role_str).map_err(|e| CoreError::Internal(e.to_string()))?;
            let is_active: i64 = row.try_get("is_active")?;
            out.push(UserPublic {
                id: row.try_get("id")?,
                full_name: row.try_get("full_name")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                role,
                is_active: is_active != 0,
                last_login_at: row.try_get("last_login_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(out)
    }

    pub async fn create_user(
        &self,
        full_name: &str,
        email: &str,
        username: Option<&str>,
        password: &str,
        role: Role,
    ) -> Result<UserPublic, CoreError> {
        let full_name = full_name.trim();
        let email = email.trim().to_ascii_lowercase();
        let username = username
            .map(|u| u.trim().to_ascii_lowercase())
            .filter(|u| !u.is_empty());

        if full_name.len() < 2 {
            return Err(CoreError::Validation(
                "Full name must be at least 2 characters.".to_string(),
            ));
        }
        if !email.contains('@') {
            return Err(CoreError::Validation("Invalid email address.".to_string()));
        }
        validate_password_strength(password)?;

        let password_hash = hash_password(password.to_string()).await?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::now_v7().to_string();

        sqlx::query(
            "INSERT INTO users (id, full_name, email, username, password_hash, role, is_active, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)",
        )
        .bind(&id)
        .bind(full_name)
        .bind(&email)
        .bind(&username)
        .bind(&password_hash)
        .bind(role.as_str())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CoreError::Conflict("A user with that email or username already exists.".to_string())
            } else {
                CoreError::Database(e)
            }
        })?;

        Ok(UserPublic {
            id,
            full_name: full_name.to_string(),
            email,
            username,
            role,
            is_active: true,
            last_login_at: None,
            created_at: now,
        })
    }

    pub async fn delete_user(&self, target_id: &str) -> Result<(), CoreError> {
        let target: Option<(String,)> = sqlx::query_as("SELECT role FROM users WHERE id = ?")
            .bind(target_id)
            .fetch_optional(&self.pool)
            .await?;

        let Some((role_str,)) = target else {
            return Err(CoreError::NotFound("User not found.".to_string()));
        };

        if role_str == Role::SuperAdmin.as_str() {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE role = 'super_admin' AND is_active = 1",
            )
            .fetch_one(&self.pool)
            .await?;
            if count <= 1 {
                return Err(CoreError::Validation(
                    "Cannot delete the last active Super Administrator.".to_string(),
                ));
            }
        }

        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_settings(&self) -> Result<AppSettingsDto, CoreError> {
        let rows = sqlx::query("SELECT key, value_json FROM app_settings")
            .fetch_all(&self.pool)
            .await?;

        let mut dto = AppSettingsDto::default();
        for row in rows {
            let key: String = row.try_get("key")?;
            let val: String = row.try_get("value_json")?;
            match key.as_str() {
                "general.app_name" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.app_name = v;
                    }
                }
                "general.app_url" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.app_url = v;
                    }
                }
                "general.language" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.language = v;
                    }
                }
                "general.timezone" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.timezone = v;
                    }
                }
                "general.date_format" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.date_format = v;
                    }
                }
                "general.time_format" => {
                    if let Ok(v) = serde_json::from_str::<String>(&val) {
                        dto.time_format = v;
                    }
                }
                "monitoring.interval_secs" => {
                    if let Ok(v) = serde_json::from_str::<u32>(&val) {
                        dto.monitoring_interval_secs = v;
                    }
                }
                "security.session_timeout_hours" => {
                    if let Ok(v) = serde_json::from_str::<u32>(&val) {
                        dto.session_timeout_hours = v;
                    }
                }
                "security.max_login_attempts" => {
                    if let Ok(v) = serde_json::from_str::<u32>(&val) {
                        dto.max_login_attempts = v;
                    }
                }
                "network.listen_port" => {
                    if let Ok(v) = serde_json::from_str::<u16>(&val) {
                        dto.listen_port = v;
                    }
                }
                "security.audit_retention_days" => {
                    if let Ok(v) = serde_json::from_str::<u32>(&val) {
                        dto.audit_retention_days = v;
                    }
                }
                _ => {}
            }
        }
        Ok(dto)
    }

    pub async fn update_settings(
        &self,
        dto: &AppSettingsDto,
        actor_user_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if dto.listen_port < 1024 {
            return Err(CoreError::Validation(
                "Listen port must be between 1024 and 65535.".to_string(),
            ));
        }
        if !matches!(dto.language.as_str(), "en" | "id") {
            return Err(CoreError::Validation(
                "Language must be 'en' or 'id'.".to_string(),
            ));
        }

        let entries: Vec<(&str, serde_json::Value)> = vec![
            ("general.app_name", serde_json::json!(dto.app_name.trim())),
            ("general.app_url", serde_json::json!(dto.app_url.trim())),
            ("general.language", serde_json::json!(dto.language)),
            ("general.timezone", serde_json::json!(dto.timezone.trim())),
            ("general.date_format", serde_json::json!(dto.date_format)),
            ("general.time_format", serde_json::json!(dto.time_format)),
            (
                "monitoring.interval_secs",
                serde_json::json!(dto.monitoring_interval_secs.clamp(2, 300)),
            ),
            (
                "security.session_timeout_hours",
                serde_json::json!(dto.session_timeout_hours.clamp(1, 720)),
            ),
            (
                "security.max_login_attempts",
                serde_json::json!(dto.max_login_attempts.clamp(3, 20)),
            ),
            ("network.listen_port", serde_json::json!(dto.listen_port)),
            (
                "security.audit_retention_days",
                serde_json::json!(dto.audit_retention_days.clamp(7, 3650)),
            ),
        ];

        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        for (key, value) in entries {
            sqlx::query(
                "INSERT INTO app_settings (key, value_json, is_sensitive, updated_by, updated_at)
                 VALUES (?, ?, 0, ?, ?)
                 ON CONFLICT(key) DO UPDATE SET
                   value_json = excluded.value_json,
                   updated_by = excluded.updated_by,
                   updated_at = excluded.updated_at",
            )
            .bind(key)
            .bind(value.to_string())
            .bind(actor_user_id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_servers(&self) -> Result<Vec<ServerSummary>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, name, group_name, connection_type, endpoint, status, docker_version,
                    os_info, cpu_cores, memory_total_bytes, last_seen_at, created_at
             FROM servers ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(ServerSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                group_name: row.try_get("group_name")?,
                connection_type: row.try_get("connection_type")?,
                endpoint: row.try_get("endpoint")?,
                status: row.try_get("status")?,
                docker_version: row.try_get("docker_version")?,
                os_info: row.try_get("os_info")?,
                cpu_cores: row.try_get("cpu_cores")?,
                memory_total_bytes: row.try_get("memory_total_bytes")?,
                last_seen_at: row.try_get("last_seen_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(out)
    }

    pub async fn register_local_server(
        &self,
        name: &str,
        endpoint: &str,
        docker_version: Option<&str>,
        os_info: Option<&str>,
        online: bool,
    ) -> Result<ServerSummary, CoreError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let status = if online { "online" } else { "offline" };

        sqlx::query(
            "INSERT INTO servers (id, name, group_name, connection_type, endpoint, status, docker_version, os_info, last_seen_at, created_at, updated_at)
             VALUES (?, ?, 'Default', 'local_socket', ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(endpoint)
        .bind(status)
        .bind(docker_version)
        .bind(os_info)
        .bind(if online { Some(&now) } else { None })
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CoreError::Conflict("A server with that name already exists.".to_string())
            } else {
                CoreError::Database(e)
            }
        })?;

        Ok(ServerSummary {
            id,
            name: name.trim().to_string(),
            group_name: "Default".to_string(),
            connection_type: "local_socket".to_string(),
            endpoint: Some(endpoint.to_string()),
            status: status.to_string(),
            docker_version: docker_version.map(|s| s.to_string()),
            os_info: os_info.map(|s| s.to_string()),
            cpu_cores: None,
            memory_total_bytes: None,
            last_seen_at: if online { Some(now.clone()) } else { None },
            created_at: now,
        })
    }

    pub async fn create_remote_server_with_token(
        &self,
        name: &str,
        group_name: &str,
        actor_user_id: &str,
    ) -> Result<(ServerSummary, String, String), CoreError> {
        let server_id = Uuid::now_v7().to_string();
        let token_id = Uuid::now_v7().to_string();
        let raw_token = format!("mto_reg_{}", generate_opaque_token());
        let token_hash = hash_token(&raw_token);
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let expires_at = (now + Duration::minutes(15)).to_rfc3339();

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO servers (id, name, group_name, connection_type, status, created_at, updated_at)
             VALUES (?, ?, ?, 'remote_agent', 'pending', ?, ?)",
        )
        .bind(&server_id)
        .bind(name.trim())
        .bind(if group_name.trim().is_empty() {
            "Default"
        } else {
            group_name.trim()
        })
        .bind(&now_str)
        .bind(&now_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CoreError::Conflict("A server with that name already exists.".to_string())
            } else {
                CoreError::Database(e)
            }
        })?;

        sqlx::query(
            "INSERT INTO agent_registration_tokens (id, server_id, token_hash, expires_at, created_by, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&token_id)
        .bind(&server_id)
        .bind(&token_hash)
        .bind(&expires_at)
        .bind(actor_user_id)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let server = ServerSummary {
            id: server_id,
            name: name.trim().to_string(),
            group_name: if group_name.trim().is_empty() {
                "Default".to_string()
            } else {
                group_name.trim().to_string()
            },
            connection_type: "remote_agent".to_string(),
            endpoint: None,
            status: "pending".to_string(),
            docker_version: None,
            os_info: None,
            cpu_cores: None,
            memory_total_bytes: None,
            last_seen_at: None,
            created_at: now_str,
        };

        Ok((server, raw_token, expires_at))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn write_audit_log(
        &self,
        actor_user_id: Option<&str>,
        actor_email: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        server_id: Option<&str>,
        status: &str,
        ip_address: Option<&str>,
        details: serde_json::Value,
    ) -> Result<(), CoreError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO audit_logs (id, actor_user_id, actor_email, action, resource_type, resource_id, server_id, status, ip_address, details_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(actor_user_id)
        .bind(actor_email)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(server_id)
        .bind(status)
        .bind(ip_address)
        .bind(details.to_string())
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_audit_logs(&self, limit: i64) -> Result<Vec<AuditLogEntry>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, actor_user_id, actor_email, action, resource_type, resource_id,
                    server_id, status, ip_address, details_json, created_at
             FROM audit_logs
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let details_str: String = row.try_get("details_json")?;
            let details = serde_json::from_str(&details_str).unwrap_or(serde_json::json!({}));
            out.push(AuditLogEntry {
                id: row.try_get("id")?,
                actor_user_id: row.try_get("actor_user_id")?,
                actor_email: row.try_get("actor_email")?,
                action: row.try_get("action")?,
                resource_type: row.try_get("resource_type")?,
                resource_id: row.try_get("resource_id")?,
                server_id: row.try_get("server_id")?,
                status: row.try_get("status")?,
                ip_address: row.try_get("ip_address")?,
                details,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(out)
    }

    pub async fn insert_host_metric(&self, point: &HostMetricPoint) -> Result<(), CoreError> {
        sqlx::query(
            "INSERT INTO metrics_host (
                server_id, cpu_percent, mem_used_bytes, mem_total_bytes, mem_percent,
                disk_used_bytes, disk_total_bytes, disk_percent,
                disk_active_percent, disk_read_bps, disk_write_bps,
                net_rx_bps, net_tx_bps, uptime_seconds, recorded_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&point.server_id)
        .bind(point.cpu_percent)
        .bind(point.mem_used_bytes)
        .bind(point.mem_total_bytes)
        .bind(point.mem_percent)
        .bind(point.disk_used_bytes)
        .bind(point.disk_total_bytes)
        .bind(point.disk_percent)
        .bind(point.disk_active_percent)
        .bind(point.disk_read_bps)
        .bind(point.disk_write_bps)
        .bind(point.net_rx_bps)
        .bind(point.net_tx_bps)
        .bind(point.uptime_seconds)
        .bind(&point.recorded_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn query_host_metrics(
        &self,
        server_id: &str,
        since_minutes: i64,
        limit: i64,
    ) -> Result<Vec<HostMetricPoint>, CoreError> {
        let since = (Utc::now() - Duration::minutes(since_minutes.max(1))).to_rfc3339();
        let rows = sqlx::query(
            "SELECT server_id, cpu_percent, mem_used_bytes, mem_total_bytes, mem_percent,
                    disk_used_bytes, disk_total_bytes, disk_percent,
                    disk_active_percent, disk_read_bps, disk_write_bps,
                    net_rx_bps, net_tx_bps, uptime_seconds, recorded_at
             FROM metrics_host
             WHERE server_id = ? AND recorded_at >= ?
             ORDER BY recorded_at DESC
             LIMIT ?",
        )
        .bind(server_id)
        .bind(&since)
        .bind(limit.clamp(10, 1000))
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(HostMetricPoint {
                server_id: row.try_get("server_id")?,
                cpu_percent: row.try_get("cpu_percent")?,
                mem_used_bytes: row.try_get("mem_used_bytes")?,
                mem_total_bytes: row.try_get("mem_total_bytes")?,
                mem_percent: row.try_get("mem_percent")?,
                disk_used_bytes: row.try_get("disk_used_bytes")?,
                disk_total_bytes: row.try_get("disk_total_bytes")?,
                disk_percent: row.try_get("disk_percent")?,
                disk_active_percent: row.try_get("disk_active_percent")?,
                disk_read_bps: row.try_get("disk_read_bps")?,
                disk_write_bps: row.try_get("disk_write_bps")?,
                net_rx_bps: row.try_get("net_rx_bps")?,
                net_tx_bps: row.try_get("net_tx_bps")?,
                uptime_seconds: row.try_get("uptime_seconds")?,
                recorded_at: row.try_get("recorded_at")?,
            });
        }
        // Return chronologically ascending for ECharts time-series rendering
        out.reverse();
        Ok(out)
    }

    pub async fn prune_old_metrics(&self, retention_days: i64) -> Result<(), CoreError> {
        let cutoff = (Utc::now() - Duration::days(retention_days.max(1))).to_rfc3339();
        sqlx::query("DELETE FROM metrics_host WHERE recorded_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Phase 3: Encrypted Secrets Vault & Service Discovery ---

    pub async fn save_encrypted_secret(
        &self,
        vault: &MasterKeyVault,
        scope: &str,
        reference_id: &str,
        plaintext: &[u8],
    ) -> Result<String, CoreError> {
        let aad = format!("{scope}:{reference_id}");
        let encrypted = vault.encrypt(plaintext, &aad)?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO encrypted_secrets (id, scope, reference_id, ciphertext_blob, nonce, aad, key_version, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)
             ON CONFLICT(scope, reference_id) DO UPDATE SET
               ciphertext_blob = excluded.ciphertext_blob,
               nonce = excluded.nonce,
               aad = excluded.aad,
               updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(scope)
        .bind(reference_id)
        .bind(&encrypted.ciphertext)
        .bind(&encrypted.nonce)
        .bind(&aad)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let (stored_id,): (String,) =
            sqlx::query_as("SELECT id FROM encrypted_secrets WHERE scope = ? AND reference_id = ?")
                .bind(scope)
                .bind(reference_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(stored_id)
    }

    pub async fn load_encrypted_secret(
        &self,
        vault: &MasterKeyVault,
        scope: &str,
        reference_id: &str,
    ) -> Result<Option<Vec<u8>>, CoreError> {
        let row = sqlx::query(
            "SELECT ciphertext_blob, nonce, aad FROM encrypted_secrets WHERE scope = ? AND reference_id = ? LIMIT 1",
        )
        .bind(scope)
        .bind(reference_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let ciphertext: Vec<u8> = row.try_get("ciphertext_blob")?;
        let nonce: Vec<u8> = row.try_get("nonce")?;
        let aad: String = row.try_get("aad")?;
        let expected_aad = format!("{scope}:{reference_id}");
        if aad != expected_aad {
            return Err(CoreError::Vault(meetoops_security::VaultError::DecryptFailed));
        }
        let plain = vault.decrypt(&ciphertext, &nonce, &aad)?;
        Ok(Some(plain))
    }

    pub async fn list_services(&self) -> Result<Vec<DiscoveredServiceDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, server_id, container_id, name, service_type, confidence, readiness_stage,
                    host, port, secret_id, read_only_mode, metadata_json, last_verified_at,
                    created_at, updated_at
             FROM discovered_services
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(map_service_row(&row)?);
        }
        Ok(out)
    }

    pub async fn get_service(&self, id: &str) -> Result<DiscoveredServiceDto, CoreError> {
        let row = sqlx::query(
            "SELECT id, server_id, container_id, name, service_type, confidence, readiness_stage,
                    host, port, secret_id, read_only_mode, metadata_json, last_verified_at,
                    created_at, updated_at
             FROM discovered_services
             WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::NotFound("Service connection not found.".to_string()))?;

        map_service_row(&row)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_discovered_service(
        &self,
        server_id: &str,
        container_id: &str,
        name: &str,
        service_type: &str,
        confidence: &str,
        readiness_stage: &str,
        host: Option<&str>,
        port: Option<u16>,
        metadata: serde_json::Value,
    ) -> Result<DiscoveredServiceDto, CoreError> {
        let existing: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT id, secret_id FROM discovered_services WHERE server_id = ? AND container_id = ?",
        )
        .bind(server_id)
        .bind(container_id)
        .fetch_optional(&self.pool)
        .await?;

        let now = Utc::now().to_rfc3339();
        let port_i64 = port.map(|p| p as i64);

        if let Some((existing_id, secret_id)) = existing {
            // Preserve higher readiness stage if user already attached credentials
            if secret_id.is_none() {
                sqlx::query(
                    "UPDATE discovered_services
                     SET name = ?, service_type = ?, confidence = ?, readiness_stage = ?,
                         host = COALESCE(host, ?), port = COALESCE(port, ?),
                         metadata_json = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(name)
                .bind(service_type)
                .bind(confidence)
                .bind(readiness_stage)
                .bind(host)
                .bind(port_i64)
                .bind(metadata.to_string())
                .bind(&now)
                .bind(&existing_id)
                .execute(&self.pool)
                .await?;
            }
            return self.get_service(&existing_id).await;
        }

        let id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO discovered_services (
                id, server_id, container_id, name, service_type, confidence, readiness_stage,
                host, port, secret_id, read_only_mode, metadata_json, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, 1, ?, ?, ?)",
        )
        .bind(&id)
        .bind(server_id)
        .bind(container_id)
        .bind(name)
        .bind(service_type)
        .bind(confidence)
        .bind(readiness_stage)
        .bind(host)
        .bind(port_i64)
        .bind(metadata.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_service(&id).await
    }

    pub async fn create_manual_service(
        &self,
        name: &str,
        service_type: &str,
        host: &str,
        port: u16,
        read_only_mode: bool,
    ) -> Result<DiscoveredServiceDto, CoreError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO discovered_services (
                id, server_id, container_id, name, service_type, confidence, readiness_stage,
                host, port, secret_id, read_only_mode, metadata_json, created_at, updated_at
             ) VALUES (?, 'local', NULL, ?, ?, 'confirmed', 'connection_discovered', ?, ?, NULL, ?, '{}', ?, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(service_type)
        .bind(host.trim())
        .bind(port as i64)
        .bind(if read_only_mode { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_service(&id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_service_connection_and_readiness(
        &self,
        id: &str,
        host: &str,
        port: u16,
        secret_id: &str,
        read_only_mode: bool,
        readiness_stage: &str,
        verified: bool,
    ) -> Result<DiscoveredServiceDto, CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE discovered_services
             SET host = ?, port = ?, secret_id = ?, read_only_mode = ?,
                 readiness_stage = ?, last_verified_at = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(host.trim())
        .bind(port as i64)
        .bind(secret_id)
        .bind(if read_only_mode { 1 } else { 0 })
        .bind(readiness_stage)
        .bind(if verified { Some(&now) } else { None })
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_service(id).await
    }

    pub async fn delete_service(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM encrypted_secrets WHERE scope = 'service_connection' AND reference_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM discovered_services WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Server-Side Paginated, Filtered & Sorted Queries ---

    #[allow(clippy::too_many_arguments)]
    pub async fn query_host_metrics_paginated(
        &self,
        server_id: &str,
        risks: &[String],
        from_date: Option<&str>,
        to_date: Option<&str>,
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<HostMetricPoint>, u64), CoreError> {
        let order_col = match sort_by {
            "cpu_percent" => "cpu_percent",
            "mem_percent" => "mem_percent",
            "disk_percent" => "disk_percent",
            "disk_active_percent" => "disk_active_percent",
            "net_rx_bps" => "net_rx_bps",
            "net_tx_bps" => "net_tx_bps",
            _ => "recorded_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM metrics_host WHERE server_id = ");
        qb_count.push_bind(server_id);
        apply_metrics_filters(&mut qb_count, risks, from_date, to_date);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT server_id, cpu_percent, mem_used_bytes, mem_total_bytes, mem_percent,
                    disk_used_bytes, disk_total_bytes, disk_percent,
                    disk_active_percent, disk_read_bps, disk_write_bps,
                    net_rx_bps, net_tx_bps, uptime_seconds, recorded_at
             FROM metrics_host WHERE server_id = ",
        );
        qb.push_bind(server_id);
        apply_metrics_filters(&mut qb, risks, from_date, to_date);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(HostMetricPoint {
                server_id: row.try_get("server_id")?,
                cpu_percent: row.try_get("cpu_percent")?,
                mem_used_bytes: row.try_get("mem_used_bytes")?,
                mem_total_bytes: row.try_get("mem_total_bytes")?,
                mem_percent: row.try_get("mem_percent")?,
                disk_used_bytes: row.try_get("disk_used_bytes")?,
                disk_total_bytes: row.try_get("disk_total_bytes")?,
                disk_percent: row.try_get("disk_percent")?,
                disk_active_percent: row.try_get("disk_active_percent")?,
                disk_read_bps: row.try_get("disk_read_bps")?,
                disk_write_bps: row.try_get("disk_write_bps")?,
                net_rx_bps: row.try_get("net_rx_bps")?,
                net_tx_bps: row.try_get("net_tx_bps")?,
                uptime_seconds: row.try_get("uptime_seconds")?,
                recorded_at: row.try_get("recorded_at")?,
            });
        }
        Ok((out, total.max(0) as u64))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_audit_logs_paginated(
        &self,
        search: Option<&str>,
        statuses: &[String],
        resources: &[String],
        from_date: Option<&str>,
        to_date: Option<&str>,
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<AuditLogEntry>, u64), CoreError> {
        let order_col = match sort_by {
            "action" => "action",
            "actor_email" => "actor_email",
            "resource_type" => "resource_type",
            "status" => "status",
            _ => "created_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM audit_logs WHERE 1=1");
        apply_audit_filters(&mut qb_count, search, statuses, resources, from_date, to_date);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, actor_user_id, actor_email, action, resource_type, resource_id,
                    server_id, status, ip_address, details_json, created_at
             FROM audit_logs WHERE 1=1",
        );
        apply_audit_filters(&mut qb, search, statuses, resources, from_date, to_date);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let details_str: String = row.try_get("details_json")?;
            let details = serde_json::from_str(&details_str).unwrap_or(serde_json::json!({}));
            out.push(AuditLogEntry {
                id: row.try_get("id")?,
                actor_user_id: row.try_get("actor_user_id")?,
                actor_email: row.try_get("actor_email")?,
                action: row.try_get("action")?,
                resource_type: row.try_get("resource_type")?,
                resource_id: row.try_get("resource_id")?,
                server_id: row.try_get("server_id")?,
                status: row.try_get("status")?,
                ip_address: row.try_get("ip_address")?,
                details,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok((out, total.max(0) as u64))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_users_paginated(
        &self,
        search: Option<&str>,
        roles: &[String],
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<UserPublic>, u64), CoreError> {
        let order_col = match sort_by {
            "full_name" => "full_name",
            "email" => "email",
            "role" => "role",
            _ => "created_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM users WHERE 1=1");
        apply_users_filters(&mut qb_count, search, roles);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, full_name, email, username, role, is_active, last_login_at, created_at
             FROM users WHERE 1=1",
        );
        apply_users_filters(&mut qb, search, roles);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let role_str: String = row.try_get("role")?;
            let role =
                Role::from_db_str(&role_str).map_err(|e| CoreError::Internal(e.to_string()))?;
            let is_active: i64 = row.try_get("is_active")?;
            out.push(UserPublic {
                id: row.try_get("id")?,
                full_name: row.try_get("full_name")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                role,
                is_active: is_active != 0,
                last_login_at: row.try_get("last_login_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok((out, total.max(0) as u64))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_services_paginated(
        &self,
        search: Option<&str>,
        types: &[String],
        stages: &[String],
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<DiscoveredServiceDto>, u64), CoreError> {
        let order_col = match sort_by {
            "name" => "name",
            "service_type" => "service_type",
            "confidence" => "confidence",
            "readiness_stage" => "readiness_stage",
            _ => "created_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count =
            sqlx::QueryBuilder::new("SELECT COUNT(*) FROM discovered_services WHERE 1=1");
        apply_services_filters(&mut qb_count, search, types, stages);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, server_id, container_id, name, service_type, confidence, readiness_stage,
                    host, port, secret_id, read_only_mode, metadata_json, last_verified_at,
                    created_at, updated_at
             FROM discovered_services WHERE 1=1",
        );
        apply_services_filters(&mut qb, search, types, stages);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(map_service_row(&row)?);
        }
        Ok((out, total.max(0) as u64))
    }

    // --- Phase 4: Multi-Server, Agent Registration & Alerts Engine ---

    #[allow(clippy::too_many_arguments)]
    pub async fn query_servers_paginated(
        &self,
        search: Option<&str>,
        statuses: &[String],
        conn_types: &[String],
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<ServerSummary>, u64), CoreError> {
        let order_col = match sort_by {
            "name" => "name",
            "group_name" => "group_name",
            "status" => "status",
            "connection_type" => "connection_type",
            _ => "created_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM servers WHERE 1=1");
        apply_servers_filters(&mut qb_count, search, statuses, conn_types);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, name, group_name, connection_type, endpoint, status, docker_version,
                    os_info, cpu_cores, memory_total_bytes, last_seen_at, created_at
             FROM servers WHERE 1=1",
        );
        apply_servers_filters(&mut qb, search, statuses, conn_types);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(ServerSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                group_name: row.try_get("group_name")?,
                connection_type: row.try_get("connection_type")?,
                endpoint: row.try_get("endpoint")?,
                status: row.try_get("status")?,
                docker_version: row.try_get("docker_version")?,
                os_info: row.try_get("os_info")?,
                cpu_cores: row.try_get("cpu_cores")?,
                memory_total_bytes: row.try_get("memory_total_bytes")?,
                last_seen_at: row.try_get("last_seen_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok((out, total.max(0) as u64))
    }

    pub async fn delete_server(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Exchanges a single-use, time-limited registration token for a long-lived agent secret.
    pub async fn register_agent_with_token(
        &self,
        raw_token: &str,
    ) -> Result<(String, String), CoreError> {
        let token_hash = hash_token(raw_token);
        let row = sqlx::query(
            "SELECT id, server_id, expires_at, used_at FROM agent_registration_tokens WHERE token_hash = ? LIMIT 1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::Unauthorized("Invalid registration token.".to_string()))?;

        let token_id: String = row.try_get("id")?;
        let server_id: String = row.try_get("server_id")?;
        let used_at: Option<String> = row.try_get("used_at")?;
        let expires_at: String = row.try_get("expires_at")?;

        if used_at.is_some() {
            return Err(CoreError::Unauthorized("Registration token already used.".to_string()));
        }
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&expires_at) {
            if exp.with_timezone(&Utc) <= Utc::now() {
                return Err(CoreError::Unauthorized("Registration token expired.".to_string()));
            }
        }

        let agent_secret = format!("mto_agt_{}", generate_opaque_token());
        let secret_hash = hash_token(&agent_secret);
        let now = Utc::now().to_rfc3339();

        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE agent_registration_tokens SET used_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&token_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO agent_credentials (server_id, agent_secret_hash, rotated_at, revoked_at)
             VALUES (?, ?, ?, NULL)
             ON CONFLICT(server_id) DO UPDATE SET
               agent_secret_hash = excluded.agent_secret_hash,
               rotated_at = excluded.rotated_at,
               revoked_at = NULL",
        )
        .bind(&server_id)
        .bind(&secret_hash)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE servers SET status = 'online', last_seen_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(&server_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok((server_id, agent_secret))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_agent_heartbeat(
        &self,
        server_id: &str,
        raw_secret: &str,
        docker_version: Option<&str>,
        os_info: Option<&str>,
        cpu_cores: Option<i64>,
        memory_total_bytes: Option<i64>,
        agent_version: Option<&str>,
    ) -> Result<(), CoreError> {
        let secret_hash = hash_token(raw_secret);
        let valid: Option<(String,)> = sqlx::query_as(
            "SELECT server_id FROM agent_credentials
             WHERE server_id = ? AND agent_secret_hash = ? AND revoked_at IS NULL",
        )
        .bind(server_id)
        .bind(&secret_hash)
        .fetch_optional(&self.pool)
        .await?;

        if valid.is_none() {
            return Err(CoreError::Unauthorized("Invalid or revoked agent secret.".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE servers
             SET status = 'online',
                 docker_version = COALESCE(?, docker_version),
                 os_info = COALESCE(?, os_info),
                 cpu_cores = COALESCE(?, cpu_cores),
                 memory_total_bytes = COALESCE(?, memory_total_bytes),
                 agent_version = COALESCE(?, agent_version),
                 last_seen_at = ?,
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(docker_version)
        .bind(os_info)
        .bind(cpu_cores)
        .bind(memory_total_bytes)
        .bind(agent_version)
        .bind(&now)
        .bind(&now)
        .bind(server_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_alert_rules(&self) -> Result<Vec<AlertRuleDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, name, server_id, metric_type, operator, threshold, severity, is_enabled, created_at, updated_at
             FROM alert_rules ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let enabled: i64 = row.try_get("is_enabled")?;
            out.push(AlertRuleDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                server_id: row.try_get("server_id")?,
                metric_type: row.try_get("metric_type")?,
                operator: row.try_get("operator")?,
                threshold: row.try_get("threshold")?,
                severity: row.try_get("severity")?,
                is_enabled: enabled != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(out)
    }

    pub async fn create_alert_rule(
        &self,
        name: &str,
        metric_type: &str,
        operator: &str,
        threshold: f64,
        severity: &str,
    ) -> Result<AlertRuleDto, CoreError> {
        if name.trim().len() < 2 {
            return Err(CoreError::Validation("Rule name must be at least 2 characters.".to_string()));
        }
        if !matches!(
            metric_type,
            "cpu_percent" | "mem_percent" | "disk_percent" | "disk_active_percent"
        ) {
            return Err(CoreError::Validation("Unsupported metric type.".to_string()));
        }
        if !matches!(operator, "gt" | "gte" | "lt") {
            return Err(CoreError::Validation("Operator must be gt, gte, or lt.".to_string()));
        }
        if !matches!(severity, "info" | "warning" | "critical") {
            return Err(CoreError::Validation("Severity must be info, warning, or critical.".to_string()));
        }

        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO alert_rules (id, name, server_id, metric_type, operator, threshold, severity, is_enabled, created_at, updated_at)
             VALUES (?, ?, 'local', ?, ?, ?, ?, 1, ?, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(metric_type)
        .bind(operator)
        .bind(threshold)
        .bind(severity)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(AlertRuleDto {
            id,
            name: name.trim().to_string(),
            server_id: "local".to_string(),
            metric_type: metric_type.to_string(),
            operator: operator.to_string(),
            threshold,
            severity: severity.to_string(),
            is_enabled: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn toggle_alert_rule(&self, id: &str, enabled: bool) -> Result<(), CoreError> {
        sqlx::query("UPDATE alert_rules SET is_enabled = ?, updated_at = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_alert_rule(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM alert_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_alert_events_paginated(
        &self,
        search: Option<&str>,
        severities: &[String],
        statuses: &[String],
        from_date: Option<&str>,
        to_date: Option<&str>,
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<AlertEventDto>, u64), CoreError> {
        let order_col = match sort_by {
            "rule_name" => "rule_name",
            "severity" => "severity",
            "status" => "status",
            "metric_value" => "metric_value",
            _ => "triggered_at",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let mut qb_count = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM alert_events WHERE 1=1");
        apply_alert_events_filters(&mut qb_count, search, severities, statuses, from_date, to_date);
        let (total,): (i64,) = qb_count.build_query_as().fetch_one(&self.pool).await?;

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, rule_id, rule_name, server_id, status, severity, message, metric_value,
                    triggered_at, acknowledged_by, acknowledged_at, resolved_at
             FROM alert_events WHERE 1=1",
        );
        apply_alert_events_filters(&mut qb, search, severities, statuses, from_date, to_date);
        qb.push(format!(" ORDER BY {order_col} {order_dir} LIMIT "));
        let limit = per_page.clamp(1, 200) as i64;
        let offset = ((page.max(1) - 1) as i64) * limit;
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(AlertEventDto {
                id: row.try_get("id")?,
                rule_id: row.try_get("rule_id")?,
                rule_name: row.try_get("rule_name")?,
                server_id: row.try_get("server_id")?,
                status: row.try_get("status")?,
                severity: row.try_get("severity")?,
                message: row.try_get("message")?,
                metric_value: row.try_get("metric_value")?,
                triggered_at: row.try_get("triggered_at")?,
                acknowledged_by: row.try_get("acknowledged_by")?,
                acknowledged_at: row.try_get("acknowledged_at")?,
                resolved_at: row.try_get("resolved_at")?,
            });
        }
        Ok((out, total.max(0) as u64))
    }

    pub async fn acknowledge_alert_event(&self, id: &str, actor_email: &str) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE alert_events SET status = 'acknowledged', acknowledged_by = ?, acknowledged_at = ?
             WHERE id = ? AND status = 'firing'",
        )
        .bind(actor_email)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn resolve_alert_event(&self, id: &str) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE alert_events SET status = 'resolved', resolved_at = ?
             WHERE id = ? AND status != 'resolved'",
        )
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Evaluates all enabled alert rules against a real host metric sample.
    /// Deduplicates already-firing alerts, auto-resolves when metric returns to normal,
    /// and returns newly fired alerts for outbound webhook dispatch.
    pub async fn evaluate_host_metric_alerts(
        &self,
        point: &HostMetricPoint,
    ) -> Result<Vec<AlertEventDto>, CoreError> {
        let rules = self.list_alert_rules().await?;
        let now = Utc::now().to_rfc3339();
        let mut newly_fired = Vec::new();

        for rule in rules.into_iter().filter(|r| r.is_enabled) {
            let val_opt = match rule.metric_type.as_str() {
                "cpu_percent" => point.cpu_percent,
                "mem_percent" => point.mem_percent,
                "disk_percent" => point.disk_percent,
                "disk_active_percent" => point.disk_active_percent,
                _ => None,
            };
            let Some(val) = val_opt else { continue };

            let breached = match rule.operator.as_str() {
                "gt" => val > rule.threshold,
                "gte" => val >= rule.threshold,
                "lt" => val < rule.threshold,
                _ => false,
            };

            let existing_open: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM alert_events WHERE rule_id = ? AND status IN ('firing', 'acknowledged') LIMIT 1",
            )
            .bind(&rule.id)
            .fetch_optional(&self.pool)
            .await?;

            if breached && existing_open.is_none() {
                let ev_id = Uuid::now_v7().to_string();
                let msg = format!(
                    "{} reached {:.1}% (threshold: {} {:.1}%)",
                    rule.metric_type, val, rule.operator, rule.threshold
                );
                sqlx::query(
                    "INSERT INTO alert_events (id, rule_id, rule_name, server_id, status, severity, message, metric_value, triggered_at)
                     VALUES (?, ?, ?, ?, 'firing', ?, ?, ?, ?)",
                )
                .bind(&ev_id)
                .bind(&rule.id)
                .bind(&rule.name)
                .bind(&point.server_id)
                .bind(&rule.severity)
                .bind(&msg)
                .bind(val)
                .bind(&now)
                .execute(&self.pool)
                .await?;

                newly_fired.push(AlertEventDto {
                    id: ev_id,
                    rule_id: rule.id,
                    rule_name: rule.name,
                    server_id: point.server_id.clone(),
                    status: "firing".to_string(),
                    severity: rule.severity,
                    message: msg,
                    metric_value: Some(val),
                    triggered_at: now.clone(),
                    acknowledged_by: None,
                    acknowledged_at: None,
                    resolved_at: None,
                });
            } else if !breached {
                if let Some((ev_id,)) = existing_open {
                    sqlx::query(
                        "UPDATE alert_events SET status = 'resolved', resolved_at = ? WHERE id = ?",
                    )
                    .bind(&now)
                    .bind(&ev_id)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }
        Ok(newly_fired)
    }

    pub async fn get_active_alerts_summary(&self) -> Result<(i64, i64), CoreError> {
        let (firing,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM alert_events WHERE status = 'firing'")
                .fetch_one(&self.pool)
                .await?;
        let (acknowledged,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM alert_events WHERE status = 'acknowledged'")
                .fetch_one(&self.pool)
                .await?;
        Ok((firing, acknowledged))
    }

    pub async fn list_alert_channels(&self) -> Result<Vec<AlertChannelDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, name, channel_type, target_url, is_enabled, created_at
             FROM alert_channels ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let enabled: i64 = row.try_get("is_enabled")?;
            out.push(AlertChannelDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                channel_type: row.try_get("channel_type")?,
                target_url: row.try_get("target_url")?,
                is_enabled: enabled != 0,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(out)
    }

    pub async fn get_alert_channel(&self, id: &str) -> Result<AlertChannelDto, CoreError> {
        let row = sqlx::query(
            "SELECT id, name, channel_type, target_url, is_enabled, created_at
             FROM alert_channels WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::NotFound("Alert channel not found.".to_string()))?;

        let enabled: i64 = row.try_get("is_enabled")?;
        Ok(AlertChannelDto {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            channel_type: row.try_get("channel_type")?,
            target_url: row.try_get("target_url")?,
            is_enabled: enabled != 0,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn create_alert_channel(
        &self,
        name: &str,
        channel_type: &str,
        target_url: &str,
    ) -> Result<AlertChannelDto, CoreError> {
        if name.trim().len() < 2 {
            return Err(CoreError::Validation(
                "Channel name must be at least 2 characters.".to_string(),
            ));
        }
        if !matches!(channel_type, "webhook" | "slack" | "discord" | "telegram") {
            return Err(CoreError::Validation(
                "Channel type must be webhook, slack, discord, or telegram.".to_string(),
            ));
        }
        let url = target_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(CoreError::Validation(
                "Target URL must start with http:// or https://".to_string(),
            ));
        }

        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO alert_channels (id, name, channel_type, target_url, is_enabled, created_at)
             VALUES (?, ?, ?, ?, 1, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(channel_type)
        .bind(url)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(AlertChannelDto {
            id,
            name: name.trim().to_string(),
            channel_type: channel_type.to_string(),
            target_url: url.to_string(),
            is_enabled: true,
            created_at: now,
        })
    }

    pub async fn toggle_alert_channel(&self, id: &str, enabled: bool) -> Result<(), CoreError> {
        sqlx::query("UPDATE alert_channels SET is_enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_alert_channel(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM alert_channels WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Phase 5: Personal API Tokens & SQLite Hot Backup ---

    pub async fn list_user_api_tokens(&self, user_id: &str) -> Result<Vec<ApiTokenDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, name, token_prefix, scopes_json, last_used_at, expires_at, created_at
             FROM api_tokens
             WHERE user_id = ? AND revoked_at IS NULL
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let scopes_str: String = row.try_get("scopes_json")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_str).unwrap_or_default();
            out.push(ApiTokenDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                token_prefix: row.try_get("token_prefix")?,
                scopes,
                last_used_at: row.try_get("last_used_at")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(out)
    }

    pub async fn create_api_token(
        &self,
        user_id: &str,
        name: &str,
        scopes: Vec<String>,
        expires_days: Option<i64>,
    ) -> Result<(ApiTokenDto, String), CoreError> {
        if name.trim().len() < 2 {
            return Err(CoreError::Validation(
                "Token name must be at least 2 characters.".to_string(),
            ));
        }
        let raw_token = format!("mto_{}", generate_opaque_token());
        let prefix: String = raw_token.chars().take(12).collect();
        let token_hash = hash_token(&raw_token);
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let expires_at = expires_days
            .filter(|d| *d > 0)
            .map(|d| (now + Duration::days(d.min(365))).to_rfc3339());
        let scopes_json = serde_json::to_string(&scopes).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO api_tokens (id, user_id, name, token_prefix, token_hash, scopes_json, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(name.trim())
        .bind(&prefix)
        .bind(&token_hash)
        .bind(&scopes_json)
        .bind(&expires_at)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        let dto = ApiTokenDto {
            id,
            name: name.trim().to_string(),
            token_prefix: prefix,
            scopes,
            last_used_at: None,
            expires_at,
            created_at: now_str,
        };
        Ok((dto, raw_token))
    }

    pub async fn revoke_api_token(&self, user_id: &str, token_id: &str) -> Result<(), CoreError> {
        sqlx::query("UPDATE api_tokens SET revoked_at = ? WHERE id = ? AND user_id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(token_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn resolve_api_token(&self, raw_token: &str) -> Result<Option<UserPublic>, CoreError> {
        let token_hash = hash_token(raw_token);
        let row = sqlx::query(
            "SELECT t.id AS token_id, t.expires_at, t.revoked_at,
                    u.id AS user_id, u.full_name, u.email, u.username, u.role, u.is_active,
                    u.last_login_at, u.created_at
             FROM api_tokens t
             JOIN users u ON u.id = t.user_id
             WHERE t.token_hash = ?
             LIMIT 1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else { return Ok(None) };
        let revoked: Option<String> = row.try_get("revoked_at")?;
        if revoked.is_some() {
            return Ok(None);
        }
        let is_active: i64 = row.try_get("is_active")?;
        if is_active == 0 {
            return Ok(None);
        }
        let expires_at: Option<String> = row.try_get("expires_at")?;
        if let Some(ref exp_str) = expires_at {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(exp_str) {
                if exp.with_timezone(&Utc) <= Utc::now() {
                    return Ok(None);
                }
            }
        }

        let token_id: String = row.try_get("token_id")?;
        let _ = sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(&token_id)
            .execute(&self.pool)
            .await;

        let role_str: String = row.try_get("role")?;
        let role = Role::from_db_str(&role_str).map_err(|e| CoreError::Internal(e.to_string()))?;

        Ok(Some(UserPublic {
            id: row.try_get("user_id")?,
            full_name: row.try_get("full_name")?,
            email: row.try_get("email")?,
            username: row.try_get("username")?,
            role,
            is_active: true,
            last_login_at: row.try_get("last_login_at")?,
            created_at: row.try_get("created_at")?,
        }))
    }

    /// Creates a consistent online SQLite snapshot using `VACUUM INTO`.
    pub async fn create_sqlite_snapshot(&self, target_path: &std::path::Path) -> Result<(), CoreError> {
        let escaped = target_path
            .to_string_lossy()
            .replace('\'', "''");
        let sql = format!("VACUUM INTO '{escaped}'");
        sqlx::query(&sql).execute(&self.pool).await?;
        Ok(())
    }

    // --- Phase 6: SSH Hosts & GitHub Projects ---

    pub async fn list_ssh_hosts(&self) -> Result<Vec<SshHostDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, name, host, port, username, auth_type, use_sudo, provisioned_user,
                    last_banner, last_connected_at, created_at, updated_at
             FROM ssh_hosts ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let use_sudo: i64 = row.try_get("use_sudo")?;
            out.push(SshHostDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                host: row.try_get("host")?,
                port: row.try_get("port")?,
                username: row.try_get("username")?,
                auth_type: row.try_get("auth_type")?,
                use_sudo: use_sudo != 0,
                provisioned_user: row.try_get("provisioned_user")?,
                last_banner: row.try_get("last_banner")?,
                last_connected_at: row.try_get("last_connected_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(out)
    }

    pub async fn get_ssh_host(&self, id: &str) -> Result<SshHostDto, CoreError> {
        let row = sqlx::query(
            "SELECT id, name, host, port, username, auth_type, use_sudo, provisioned_user,
                    last_banner, last_connected_at, created_at, updated_at
             FROM ssh_hosts WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::NotFound("SSH host not found.".to_string()))?;

        let use_sudo: i64 = row.try_get("use_sudo")?;
        Ok(SshHostDto {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            host: row.try_get("host")?,
            port: row.try_get("port")?,
            username: row.try_get("username")?,
            auth_type: row.try_get("auth_type")?,
            use_sudo: use_sudo != 0,
            provisioned_user: row.try_get("provisioned_user")?,
            last_banner: row.try_get("last_banner")?,
            last_connected_at: row.try_get("last_connected_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_ssh_host(
        &self,
        name: &str,
        host: &str,
        port: u16,
        username: &str,
        auth_type: &str,
        use_sudo: bool,
    ) -> Result<SshHostDto, CoreError> {
        if name.trim().len() < 2 {
            return Err(CoreError::Validation(
                "SSH host name must be at least 2 characters.".to_string(),
            ));
        }
        if host.trim().is_empty() || username.trim().is_empty() {
            return Err(CoreError::Validation(
                "Host and username are required.".to_string(),
            ));
        }
        if !matches!(auth_type, "password" | "private_key") {
            return Err(CoreError::Validation(
                "Auth type must be 'password' or 'private_key'.".to_string(),
            ));
        }
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO ssh_hosts (id, name, host, port, username, auth_type, use_sudo, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(host.trim())
        .bind(port as i64)
        .bind(username.trim())
        .bind(auth_type)
        .bind(if use_sudo { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CoreError::Conflict("An SSH host with that name already exists.".to_string())
            } else {
                CoreError::Database(e)
            }
        })?;
        self.get_ssh_host(&id).await
    }

    pub async fn update_ssh_host_banner(&self, id: &str, banner: &str) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE ssh_hosts SET last_banner = ?, last_connected_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(banner)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_ssh_host_provisioned_user(
        &self,
        id: &str,
        username: &str,
    ) -> Result<SshHostDto, CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE ssh_hosts SET username = ?, auth_type = 'password', use_sudo = 1, provisioned_user = ?, updated_at = ? WHERE id = ?",
        )
        .bind(username)
        .bind(username)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.get_ssh_host(id).await
    }

    pub async fn delete_ssh_host(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM encrypted_secrets WHERE scope = 'ssh_host' AND reference_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM ssh_hosts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT p.id, p.name, p.ssh_host_id, COALESCE(s.name, 'Unknown') AS ssh_host_name,
                    p.repo_url, p.branch, p.deploy_path, p.deploy_command, p.compose_override,
                    p.has_git_token, p.has_env_vars, p.last_deployed_commit, p.remote_head_commit,
                    p.last_status, p.last_deployed_at, p.created_at, p.updated_at
             FROM projects p
             LEFT JOIN ssh_hosts s ON s.id = p.ssh_host_id
             ORDER BY p.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(map_project_row(&row)?);
        }
        Ok(out)
    }

    pub async fn get_project(&self, id: &str) -> Result<ProjectDto, CoreError> {
        let row = sqlx::query(
            "SELECT p.id, p.name, p.ssh_host_id, COALESCE(s.name, 'Unknown') AS ssh_host_name,
                    p.repo_url, p.branch, p.deploy_path, p.deploy_command, p.compose_override,
                    p.has_git_token, p.has_env_vars, p.last_deployed_commit, p.remote_head_commit,
                    p.last_status, p.last_deployed_at, p.created_at, p.updated_at
             FROM projects p
             LEFT JOIN ssh_hosts s ON s.id = p.ssh_host_id
             WHERE p.id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::NotFound("Project not found.".to_string()))?;

        map_project_row(&row)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_project(
        &self,
        name: &str,
        ssh_host_id: &str,
        repo_url: &str,
        branch: &str,
        deploy_path: &str,
        deploy_command: &str,
        compose_override: Option<&str>,
        has_git_token: bool,
        has_env_vars: bool,
    ) -> Result<ProjectDto, CoreError> {
        if name.trim().len() < 2 {
            return Err(CoreError::Validation(
                "Project name must be at least 2 characters.".to_string(),
            ));
        }
        if repo_url.trim().is_empty() || deploy_path.trim().is_empty() {
            return Err(CoreError::Validation(
                "Repository URL and deploy path are required.".to_string(),
            ));
        }
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let branch_clean = if branch.trim().is_empty() { "main" } else { branch.trim() };
        let cmd_clean = if deploy_command.trim().is_empty() {
            "docker compose up -d --build"
        } else {
            deploy_command.trim()
        };

        sqlx::query(
            "INSERT INTO projects (
                id, name, ssh_host_id, repo_url, branch, deploy_path, deploy_command,
                compose_override, has_git_token, has_env_vars, last_status, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'idle', ?, ?)",
        )
        .bind(&id)
        .bind(name.trim())
        .bind(ssh_host_id)
        .bind(repo_url.trim())
        .bind(branch_clean)
        .bind(deploy_path.trim())
        .bind(cmd_clean)
        .bind(compose_override.filter(|s| !s.trim().is_empty()))
        .bind(if has_git_token { 1 } else { 0 })
        .bind(if has_env_vars { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                CoreError::Conflict("A project with that name already exists.".to_string())
            } else {
                CoreError::Database(e)
            }
        })?;

        self.get_project(&id).await
    }

    pub async fn update_project_config(
        &self,
        id: &str,
        branch: &str,
        deploy_command: &str,
        compose_override: Option<&str>,
        has_git_token: Option<bool>,
        has_env_vars: Option<bool>,
    ) -> Result<ProjectDto, CoreError> {
        let existing = self.get_project(id).await?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE projects
             SET branch = ?, deploy_command = ?, compose_override = ?,
                 has_git_token = ?, has_env_vars = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(if branch.trim().is_empty() { &existing.branch } else { branch.trim() })
        .bind(if deploy_command.trim().is_empty() { &existing.deploy_command } else { deploy_command.trim() })
        .bind(compose_override.filter(|s| !s.trim().is_empty()))
        .bind(if has_git_token.unwrap_or(existing.has_git_token) { 1 } else { 0 })
        .bind(if has_env_vars.unwrap_or(existing.has_env_vars) { 1 } else { 0 })
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.get_project(id).await
    }

    pub async fn update_project_git_status(
        &self,
        id: &str,
        remote_head_commit: Option<&str>,
        last_deployed_commit: Option<&str>,
        last_status: Option<&str>,
    ) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        if let Some(remote) = remote_head_commit {
            sqlx::query("UPDATE projects SET remote_head_commit = ?, updated_at = ? WHERE id = ?")
                .bind(remote)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(deployed) = last_deployed_commit {
            sqlx::query(
                "UPDATE projects SET last_deployed_commit = ?, last_deployed_at = ?, updated_at = ? WHERE id = ?",
            )
            .bind(deployed)
            .bind(&now)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        if let Some(st) = last_status {
            sqlx::query("UPDATE projects SET last_status = ?, updated_at = ? WHERE id = ?")
                .bind(st)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn delete_project(&self, id: &str) -> Result<(), CoreError> {
        sqlx::query(
            "DELETE FROM encrypted_secrets WHERE scope IN ('project_git_token', 'project_env') AND reference_id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_project_deployment(
        &self,
        project_id: &str,
        action: &str,
        triggered_by: Option<&str>,
    ) -> Result<String, CoreError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO project_deployments (id, project_id, action, status, triggered_by, log_output, started_at)
             VALUES (?, ?, ?, 'running', ?, '', ?)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(action)
        .bind(triggered_by)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn finish_project_deployment(
        &self,
        deployment_id: &str,
        status: &str,
        commit_hash: Option<&str>,
        log_output: &str,
    ) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE project_deployments
             SET status = ?, commit_hash = ?, log_output = ?, finished_at = ?
             WHERE id = ?",
        )
        .bind(status)
        .bind(commit_hash)
        .bind(log_output)
        .bind(&now)
        .bind(deployment_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_project_deployments(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectDeploymentDto>, CoreError> {
        let rows = sqlx::query(
            "SELECT id, project_id, action, status, commit_hash, triggered_by, log_output, started_at, finished_at
             FROM project_deployments
             WHERE project_id = ?
             ORDER BY started_at DESC
             LIMIT ?",
        )
        .bind(project_id)
        .bind(limit.clamp(1, 50))
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(ProjectDeploymentDto {
                id: row.try_get("id")?,
                project_id: row.try_get("project_id")?,
                action: row.try_get("action")?,
                status: row.try_get("status")?,
                commit_hash: row.try_get("commit_hash")?,
                triggered_by: row.try_get("triggered_by")?,
                log_output: row.try_get("log_output")?,
                started_at: row.try_get("started_at")?,
                finished_at: row.try_get("finished_at")?,
            });
        }
        Ok(out)
    }
}

fn map_project_row(row: &sqlx::sqlite::SqliteRow) -> Result<ProjectDto, CoreError> {
    let has_git_token: i64 = row.try_get("has_git_token")?;
    let has_env_vars: i64 = row.try_get("has_env_vars")?;
    let last_deployed_commit: Option<String> = row.try_get("last_deployed_commit")?;
    let remote_head_commit: Option<String> = row.try_get("remote_head_commit")?;
    let update_available = match (&last_deployed_commit, &remote_head_commit) {
        (Some(d), Some(r)) => !d.is_empty() && !r.is_empty() && d != r,
        (None, Some(r)) => !r.is_empty(),
        _ => false,
    };

    Ok(ProjectDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        ssh_host_id: row.try_get("ssh_host_id")?,
        ssh_host_name: row.try_get("ssh_host_name")?,
        repo_url: row.try_get("repo_url")?,
        branch: row.try_get("branch")?,
        deploy_path: row.try_get("deploy_path")?,
        deploy_command: row.try_get("deploy_command")?,
        compose_override: row.try_get("compose_override")?,
        has_git_token: has_git_token != 0,
        has_env_vars: has_env_vars != 0,
        last_deployed_commit,
        remote_head_commit,
        update_available,
        last_status: row.try_get("last_status")?,
        last_deployed_at: row.try_get("last_deployed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn apply_servers_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    search: Option<&'a str>,
    statuses: &'a [String],
    conn_types: &'a [String],
) {
    if let Some(q) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR group_name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR COALESCE(endpoint, '') LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    if !statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut sep = qb.separated(", ");
        for s in statuses {
            sep.push_bind(s.as_str());
        }
        sep.push_unseparated(")");
    }
    if !conn_types.is_empty() {
        qb.push(" AND connection_type IN (");
        let mut sep = qb.separated(", ");
        for t in conn_types {
            sep.push_bind(t.as_str());
        }
        sep.push_unseparated(")");
    }
}

fn apply_alert_events_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    search: Option<&'a str>,
    severities: &'a [String],
    statuses: &'a [String],
    from_date: Option<&'a str>,
    to_date: Option<&'a str>,
) {
    if let Some(q) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (rule_name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR message LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    if !severities.is_empty() {
        qb.push(" AND severity IN (");
        let mut sep = qb.separated(", ");
        for s in severities {
            sep.push_bind(s.as_str());
        }
        sep.push_unseparated(")");
    }
    if !statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut sep = qb.separated(", ");
        for s in statuses {
            sep.push_bind(s.as_str());
        }
        sep.push_unseparated(")");
    }
    if let Some(from) = from_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND triggered_at >= ");
        qb.push_bind(format!("{from}T00:00:00Z"));
    }
    if let Some(to) = to_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND triggered_at <= ");
        qb.push_bind(format!("{to}T23:59:59.999Z"));
    }
}

fn apply_metrics_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    risks: &[String],
    from_date: Option<&'a str>,
    to_date: Option<&'a str>,
) {
    if let Some(from) = from_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND recorded_at >= ");
        qb.push_bind(format!("{from}T00:00:00Z"));
    }
    if let Some(to) = to_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND recorded_at <= ");
        qb.push_bind(format!("{to}T23:59:59.999Z"));
    }
    if !risks.is_empty() {
        let mut conds: Vec<&str> = Vec::new();
        for r in risks {
            match r.as_str() {
                "critical" => conds.push("(cpu_percent >= 85.0 OR mem_percent >= 90.0 OR disk_percent >= 90.0)"),
                "warning" => conds.push("((cpu_percent >= 70.0 AND cpu_percent < 85.0) OR (mem_percent >= 75.0 AND mem_percent < 90.0) OR (disk_percent >= 75.0 AND disk_percent < 90.0))"),
                "safe" => conds.push("(COALESCE(cpu_percent, 0) < 70.0 AND COALESCE(mem_percent, 0) < 75.0 AND COALESCE(disk_percent, 0) < 75.0)"),
                _ => {}
            }
        }
        if !conds.is_empty() {
            qb.push(" AND (");
            qb.push(conds.join(" OR "));
            qb.push(")");
        }
    }
}

fn apply_audit_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    search: Option<&'a str>,
    statuses: &'a [String],
    resources: &'a [String],
    from_date: Option<&'a str>,
    to_date: Option<&'a str>,
) {
    if let Some(q) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (action LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR COALESCE(actor_email, '') LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR resource_type LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    if !statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut sep = qb.separated(", ");
        for s in statuses {
            sep.push_bind(s.as_str());
        }
        sep.push_unseparated(")");
    }
    if !resources.is_empty() {
        qb.push(" AND resource_type IN (");
        let mut sep = qb.separated(", ");
        for r in resources {
            sep.push_bind(r.as_str());
        }
        sep.push_unseparated(")");
    }
    if let Some(from) = from_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND created_at >= ");
        qb.push_bind(format!("{from}T00:00:00Z"));
    }
    if let Some(to) = to_date.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND created_at <= ");
        qb.push_bind(format!("{to}T23:59:59.999Z"));
    }
}

fn apply_users_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    search: Option<&'a str>,
    roles: &'a [String],
) {
    if let Some(q) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (full_name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR email LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    if !roles.is_empty() {
        qb.push(" AND role IN (");
        let mut sep = qb.separated(", ");
        for r in roles {
            sep.push_bind(r.as_str());
        }
        sep.push_unseparated(")");
    }
}

fn apply_services_filters<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>,
    search: Option<&'a str>,
    types: &'a [String],
    stages: &'a [String],
) {
    if let Some(q) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR service_type LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR COALESCE(host, '') LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
    if !types.is_empty() {
        qb.push(" AND service_type IN (");
        let mut sep = qb.separated(", ");
        for t in types {
            sep.push_bind(t.as_str());
        }
        sep.push_unseparated(")");
    }
    if !stages.is_empty() {
        qb.push(" AND readiness_stage IN (");
        let mut sep = qb.separated(", ");
        for s in stages {
            sep.push_bind(s.as_str());
        }
        sep.push_unseparated(")");
    }
}

fn map_service_row(row: &sqlx::sqlite::SqliteRow) -> Result<DiscoveredServiceDto, CoreError> {
    let secret_id: Option<String> = row.try_get("secret_id")?;
    let ro: i64 = row.try_get("read_only_mode")?;
    let meta_str: String = row.try_get("metadata_json")?;
    let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));

    Ok(DiscoveredServiceDto {
        id: row.try_get("id")?,
        server_id: row.try_get("server_id")?,
        container_id: row.try_get("container_id")?,
        name: row.try_get("name")?,
        service_type: row.try_get("service_type")?,
        confidence: row.try_get("confidence")?,
        readiness_stage: row.try_get("readiness_stage")?,
        host: row.try_get("host")?,
        port: row.try_get("port")?,
        has_credentials: secret_id.is_some(),
        read_only_mode: ro != 0,
        metadata,
        last_verified_at: row.try_get("last_verified_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_database;
    use meetoops_security::MasterKeyVault;

    #[tokio::test]
    async fn setup_wizard_is_atomic_and_locks_permanently() {
        let vault = MasterKeyVault::from_bytes([7u8; 32]);
        let pool = init_in_memory_database(&vault).await.expect("init db");
        let store = CoreStore::new(pool);

        let state = store.get_app_state().await.expect("state");
        assert_eq!(state.setup_stage, SetupStage::FirstRunSetup);

        let (admin, token, _csrf) = store
            .create_initial_super_admin(
                "Root Admin",
                "admin@meetoops.local",
                Some("admin"),
                "WarmObsidian2026!",
                Some("127.0.0.1"),
                None,
            )
            .await
            .expect("create initial admin");
        assert_eq!(admin.role, Role::SuperAdmin);

        // Second attempt must fail with SetupLocked
        let second = store
            .create_initial_super_admin(
                "Attacker",
                "evil@example.com",
                None,
                "WarmObsidian2026!",
                None,
                None,
            )
            .await;
        assert!(matches!(second, Err(CoreError::SetupLocked)));

        // Session resolves
        let sess = store
            .resolve_session(&token)
            .await
            .expect("resolve")
            .expect("session exists");
        assert_eq!(sess.user.email, "admin@meetoops.local");

        // Lock permanently
        store
            .set_setup_stage(SetupStage::Ready)
            .await
            .expect("complete");
        assert!(matches!(
            store.set_setup_stage(SetupStage::FirstRunSetup).await,
            Err(CoreError::SetupLocked)
        ));
    }
}
