# MeetoOps — Configuration Architecture

MeetoOps eliminates manual `.env` file editing for everyday administration. Runtime configuration is managed directly through the web **Settings** interface and persisted safely in structured storage.

---

## 1. Where Configuration Lives

| Category | Storage Location | Examples |
|---|---|---|
| **Runtime Settings** | SQLite `app_settings` table (`meetoops_data` volume) | App name, timezone, date/time format, monitoring intervals, retention periods, alert defaults, session timeouts, password policies |
| **Infrastructure Metadata** | SQLite (`servers`, `discovered_services`, `alert_rules`) | Registered servers, agent metadata, service connection hosts/ports |
| **Sensitive Credentials** | SQLite `encrypted_secrets` table (AES-256-GCM encrypted) | Database explorer credentials, Redis passwords, webhook secrets |
| **Master Encryption Key** | Dedicated volume `/data/keys/master.key` (`meetoops_keys`) | 256-bit CSPRNG master key, auto-generated once on first boot |
| **Deployment Overrides** | Optional Environment Variables | Bind address/port, data directory path, custom TLS proxy headers |

---

## 2. Web UI Settings Sections

After initial setup, `super_admin` users manage all runtime configuration under **Settings**:
- **General**: App display name, public Application URL (used for agent commands & WS origin checks), timezone, date/time format, default monitoring interval.
- **Users & Roles**: Create/edit/disable/delete users, assign roles (`super_admin`, `admin`, `operator`, `viewer`), reset passwords.
- **Security**: Session timeout, password policy, login attempt lockouts, API tokens, agent auth/revocation, encryption vault status, audit log retention.
- **Infrastructure**: Docker connections, remote agents, metrics & log retention policies, global alert thresholds.
- **Backup & Restore**: Export/validate/restore configuration and database snapshots.

Every change to security or infrastructure settings is recorded in `audit_logs` with sensitive values masked.

---

## 3. Optional Deployment-Level Environment Variables

None of these are required for standard installation. They exist strictly for advanced operators and container orchestration overrides:

| Variable | Default | Description |
|---|---|---|
| `MEETOOPS_BIND_ADDR` | `0.0.0.0:9480` | HTTP/WS listen address inside container (overrides DB `network.listen_port`) |
| `MEETOOPS_DATA_DIR` | `/data` | Base directory for DB (`/data/db`) and config |
| `MEETOOPS_KEY_PATH` | `/data/keys/master.key` | Path to persistent master encryption key |
| `MEETOOPS_DATABASE_URL` | `sqlite:///data/db/meetoops.db` | Database connection URL (future Postgres override) |
| `MEETOOPS_DOCKER_HOST` | Auto-detected socket | Optional override (e.g., internal socket proxy URL) |
| `MEETOOPS_LOG_LEVEL` | `info` | `tracing` filter (`error`, `warn`, `info`, `debug`) |
| `MEETOOPS_TRUST_PROXY` | `false` | Set `true` behind Caddy/Nginx/Traefik for `X-Forwarded-*` & `Secure` cookies |
