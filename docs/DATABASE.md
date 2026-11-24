# MeetoOps — Database Architecture & Schema Specification

## 1. Storage Strategy

- **Default Engine**: **SQLite 3** via `sqlx` (`sqlite://...`), located in the persistent volume at `/data/db/meetoops.db`.
- **SQLite Pragmas Applied on Every Connection**:
  - `PRAGMA journal_mode = WAL;` (concurrent readers + non-blocking writes)
  - `PRAGMA foreign_keys = ON;` (strict referential integrity)
  - `PRAGMA synchronous = NORMAL;` (safe durability with high throughput in WAL mode)
  - `PRAGMA busy_timeout = 5000;` (prevents transient lock errors under load)
- **PostgreSQL Readiness**:
  - All persistence operations sit behind async repository traits in `crates/core/src/repo/`.
  - Primary keys use standard `TEXT` UUIDv7/UUIDv4 strings (sortable, portable across SQLite and PostgreSQL).
  - Timestamps use ISO-8601 UTC `TEXT` (`YYYY-MM-DDTHH:MM:SS.sssZ`) or Unix epoch milliseconds (`INTEGER`/`BIGINT`) consistently.
  - Booleans use `INTEGER` (`0`/`1`) mapped cleanly via SQLx types.
  - JSON payloads use `TEXT` (valid JSON) to map seamlessly to `JSONB` in PostgreSQL later.

---

## 2. Migration Strategy

- Versioned SQL migrations live in `/migrations/` (e.g., `0001_initial_schema.sql`, `0002_metrics_and_alerts.sql`).
- Embedded into the `meetoops-server` binary using `sqlx::migrate!()` and executed automatically inside a startup transaction before the HTTP listener binds.
- Every migration is **forward-only** and tracked in `_sqlx_migrations`.
- Destructive schema migrations are prohibited in minor upgrades; any major breaking migration requires an explicit pre-migration automatic SQLite backup snapshot.

---

## 3. Core Tables

### 3.1 System State & Configuration

#### `app_state` (Singleton row: `id = 1`)
Enforces the First-Run Setup state machine and prevents duplicate initialization.

| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY CHECK (id = 1)` | Enforces single-row invariant |
| `setup_stage` | `TEXT` | `NOT NULL` | `UNINITIALIZED`, `FIRST_RUN_SETUP`, `ADMIN_CREATED`, `INFRASTRUCTURE_SETUP`, `READY` |
| `initialized_at` | `TEXT` | `NULL` | UTC timestamp when setup completed |
| `schema_version` | `INTEGER` | `NOT NULL DEFAULT 1` | Application-level state version |
| `key_fingerprint` | `TEXT` | `NOT NULL` | SHA-256 fingerprint of the master encryption key (for integrity verification on boot) |
| `updated_at` | `TEXT` | `NOT NULL` | Last update timestamp |

#### `app_settings`
Runtime settings managed via the Settings UI (replaces `.env` editing).

| Column | Type | Constraints | Description |
|---|---|---|---|
| `key` | `TEXT` | `PRIMARY KEY` | Namespaced key (e.g., `general.app_name`, `monitoring.interval_secs`, `security.session_timeout_mins`) |
| `value_json` | `TEXT` | `NOT NULL` | JSON-encoded value |
| `is_sensitive` | `INTEGER` | `NOT NULL DEFAULT 0` | If `1`, masked in UI & audit diffs |
| `updated_by` | `TEXT` | `NULL REFERENCES users(id)` | Actor who last modified |
| `updated_at` | `TEXT` | `NOT NULL` | UTC timestamp |

---

### 3.2 Identity, RBAC & Sessions

#### `users`
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | UUID |
| `full_name` | `TEXT` | `NOT NULL` | Display name |
| `email` | `TEXT` | `NOT NULL UNIQUE` | Normalized lowercase email |
| `username` | `TEXT` | `NULL UNIQUE` | Optional login username |
| `password_hash` | `TEXT` | `NOT NULL` | Argon2id PHC string |
| `role` | `TEXT` | `NOT NULL` | `super_admin`, `admin`, `operator`, `viewer` |
| `is_active` | `INTEGER` | `NOT NULL DEFAULT 1` | Soft disable flag |
| `failed_login_attempts` | `INTEGER` | `NOT NULL DEFAULT 0` | Brute-force lockout counter |
| `locked_until` | `TEXT` | `NULL` | Temporary lockout expiration |
| `last_login_at` | `TEXT` | `NULL` | Last successful login |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |
| `updated_at` | `TEXT` | `NOT NULL` | UTC timestamp |

> **Atomic Super Admin Constraint**: A partial unique index or transactional check on `app_state.setup_stage` guarantees only one initial Super Administrator can be created during `FIRST_RUN_SETUP`.

#### `sessions`
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | Session UUID |
| `user_id` | `TEXT` | `NOT NULL REFERENCES users(id) ON DELETE CASCADE` | Owner |
| `token_hash` | `TEXT` | `NOT NULL UNIQUE` | SHA-256 hash of opaque session cookie token |
| `csrf_token` | `TEXT` | `NOT NULL` | Per-session CSRF token |
| `ip_address` | `TEXT` | `NULL` | Client IP |
| `user_agent` | `TEXT` | `NULL` | Client User-Agent |
| `expires_at` | `TEXT` | `NOT NULL` | Expiration timestamp |
| `revoked_at` | `TEXT` | `NULL` | Explicit revocation timestamp |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |

#### `api_tokens`
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | Token UUID |
| `user_id` | `TEXT` | `NOT NULL REFERENCES users(id) ON DELETE CASCADE` | Owner |
| `name` | `TEXT` | `NOT NULL` | Human label |
| `token_prefix` | `TEXT` | `NOT NULL` | First 8 chars for identification (`mto_...`) |
| `token_hash` | `TEXT` | `NOT NULL UNIQUE` | SHA-256 hash of full token |
| `scopes_json` | `TEXT` | `NOT NULL` | JSON array of granted permission scopes |
| `last_used_at` | `TEXT` | `NULL` | Last usage timestamp |
| `expires_at` | `TEXT` | `NULL` | Optional expiry |
| `revoked_at` | `TEXT` | `NULL` | Revocation timestamp |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |

---

### 3.3 Servers, Agents & Encrypted Secrets

#### `servers`
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | UUID |
| `name` | `TEXT` | `NOT NULL UNIQUE` | Friendly server name |
| `group_name` | `TEXT` | `NOT NULL DEFAULT 'Default'` | Environment grouping (e.g., Production, Staging) |
| `connection_type` | `TEXT` | `NOT NULL` | `local_socket`, `remote_agent` |
| `endpoint` | `TEXT` | `NULL` | Socket path (local) or agent address |
| `status` | `TEXT` | `NOT NULL DEFAULT 'unknown'` | `online`, `offline`, `degraded`, `pending`, `unknown` |
| `docker_version` | `TEXT` | `NULL` | Cached Docker Engine version |
| `os_info` | `TEXT` | `NULL` | Host OS / kernel info |
| `cpu_cores` | `INTEGER` | `NULL` | Total host CPU cores |
| `memory_total_bytes` | `INTEGER` | `NULL` | Total host RAM |
| `disk_total_bytes` | `INTEGER` | `NULL` | Total primary disk bytes |
| `agent_version` | `TEXT` | `NULL` | Remote agent version |
| `last_seen_at` | `TEXT` | `NULL` | Last heartbeat timestamp |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |
| `updated_at` | `TEXT` | `NOT NULL` | UTC timestamp |

#### `agent_credentials` & `agent_registration_tokens`
- `agent_registration_tokens`: `id`, `server_id`, `token_hash`, `expires_at`, `used_at`, `created_by`, `created_at`.
- `agent_credentials`: `server_id` (PK, FK `servers`), `agent_secret_hash`, `key_version`, `rotated_at`, `revoked_at`.

#### `encrypted_secrets`
Centralized AEAD (`AES-256-GCM`) encrypted vault for database credentials, Redis passwords, webhook secrets, etc.

| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | UUID |
| `scope` | `TEXT` | `NOT NULL` | `service_connection`, `alert_channel`, `integration` |
| `reference_id` | `TEXT` | `NOT NULL` | FK-like ID of owning resource |
| `ciphertext_blob` | `BLOB` | `NOT NULL` | Encrypted payload |
| `nonce` | `BLOB` | `NOT NULL` | Unique 96-bit random nonce per encryption |
| `aad` | `TEXT` | `NOT NULL` | Associated authenticated data (`scope:reference_id`) |
| `key_version` | `INTEGER` | `NOT NULL DEFAULT 1` | Supports future key rotation |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |
| `updated_at` | `TEXT` | `NOT NULL` | UTC timestamp |

---

### 3.4 Service Discovery & Explorer Connections

#### `discovered_services`
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | UUID |
| `server_id` | `TEXT` | `NOT NULL REFERENCES servers(id) ON DELETE CASCADE` | Host server |
| `container_id` | `TEXT` | `NOT NULL` | Docker container ID |
| `service_type` | `TEXT` | `NOT NULL` | `mysql`, `mariadb`, `postgres`, `redis`, `mongodb`, `rabbitmq`, `elasticsearch`, `nginx`, `apache` |
| `confidence` | `TEXT` | `NOT NULL` | `confirmed`, `probable`, `unknown` |
| `readiness_stage` | `TEXT` | `NOT NULL` | `detected`, `connection_discovered`, `credentials_available`, `auth_verified`, `explorer_ready` |
| `host` | `TEXT` | `NULL` | Discovered host/IP |
| `port` | `INTEGER` | `NULL` | Discovered port |
| `secret_id` | `TEXT` | `NULL REFERENCES encrypted_secrets(id)` | Link to user-provided encrypted credentials |
| `read_only_mode` | `INTEGER` | `NOT NULL DEFAULT 1` | Read-only default for explorers |
| `metadata_json` | `TEXT` | `NOT NULL` | Non-secret detection signals (image, labels, ports) |
| `last_verified_at` | `TEXT` | `NULL` | Last successful auth check |
| `updated_at` | `TEXT` | `NOT NULL` | UTC timestamp |

---

### 3.5 Monitoring, Alerts & Audit Logs

#### `metrics_host` & `metrics_container`
- Time-series tables indexed by `(server_id, recorded_at)` and `(server_id, container_id, recorded_at)`.
- All metric value columns (`cpu_percent`, `mem_used_bytes`, `mem_total_bytes`, `disk_used_bytes`, `disk_read_bps`, `disk_write_bps`, `net_rx_bps`, `net_tx_bps`) are **nullable (`REAL`/`INTEGER NULL`)** so missing telemetry is stored as `NULL`, never `0`.
- Background retention worker prunes raw samples older than configured retention (default `7 days` raw, `30 days` 5-minute rollups).

#### `alert_rules` & `alert_events`
- `alert_rules`: `id`, `name`, `server_id` (nullable = global), `metric_type`, `operator`, `threshold`, `duration_secs`, `severity` (`info`/`warning`/`critical`), `is_enabled`, `created_at`.
- `alert_events`: `id`, `rule_id`, `server_id`, `container_id`, `status` (`firing`/`acknowledged`/`resolved`), `severity`, `message`, `metric_value`, `triggered_at`, `acknowledged_by`, `acknowledged_at`, `resolved_at`.

#### `audit_logs` (Append-Only)
| Column | Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | UUIDv7 (time-ordered) |
| `actor_user_id` | `TEXT` | `NULL` | User ID (null for system/unauthenticated events) |
| `actor_email` | `TEXT` | `NULL` | Snapshot of actor email |
| `action` | `TEXT` | `NOT NULL` | e.g., `setup.admin_created`, `container.remove`, `secret.reveal`, `db.query_write`, `terminal.open` |
| `resource_type` | `TEXT` | `NOT NULL` | `user`, `server`, `container`, `image`, `volume`, `network`, `database`, `redis`, `setting` |
| `resource_id` | `TEXT` | `NULL` | Target ID |
| `server_id` | `TEXT` | `NULL` | Target server |
| `status` | `TEXT` | `NOT NULL` | `success`, `denied`, `failure` |
| `ip_address` | `TEXT` | `NULL` | Client IP |
| `details_json` | `TEXT` | `NOT NULL` | Redacted structured metadata (never contains passwords/secrets) |
| `created_at` | `TEXT` | `NOT NULL` | UTC timestamp |
