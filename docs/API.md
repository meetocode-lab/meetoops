# MeetoOps — REST API & WebSocket Specification

## 1. Conventions

- **Base Path**: `/api/v1` (REST), `/ws/v1` (WebSocket).
- **Content-Type**: `application/json` (UTF-8).
- **Authentication**:
  - Browser clients: `HttpOnly`, `SameSite=Lax`, `Secure` (when HTTPS) session cookie (`meetoops_session`) + `X-CSRF-Token` header on all state-changing requests (`POST`, `PUT`, `PATCH`, `DELETE`).
  - Programmatic clients: `Authorization: Bearer mto_<token>` (exempt from CSRF, strictly scoped by token permissions).
- **Standard Success Envelope**:
  ```json
  {
    "data": { ... },
    "meta": { "page": 1, "per_page": 50, "total": 124 }
  }
  ```
- **Standard Error Envelope**:
  ```json
  {
    "error": {
      "code": "FORBIDDEN",
      "message": "Insufficient permissions for container removal.",
      "request_id": "req_01J..."
    }
  }
  ```
- **Missing Metrics Convention**: Unavailable numeric telemetry MUST be serialized as `null`, never `0`.

---

## 2. Health & First-Run Setup Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/v1/health` | Public | Liveness & readiness check (`status`, `db`, `setup_stage`, `version`) |
| `GET` | `/api/v1/setup/status` | Public | Current `setup_stage` (`UNINITIALIZED`..`READY`) and local Docker availability hint |
| `POST` | `/api/v1/setup/admin` | Public* | Atomic initial Super Admin creation (*only allowed in `FIRST_RUN_SETUP`; permanently `403` after*) |
| `POST` | `/api/v1/setup/preferences` | SuperAdmin* | Save initial app name, timezone, date/time format, monitoring interval |
| `POST` | `/api/v1/setup/docker/test-local` | SuperAdmin* | Test local Docker Engine socket and return version, OS, container/image counts |
| `POST` | `/api/v1/setup/docker/connect-local` | SuperAdmin* | Register local Docker host as first server |
| `POST` | `/api/v1/setup/docker/remote-token` | SuperAdmin* | Generate time-limited agent registration token + install command |
| `POST` | `/api/v1/setup/complete` | SuperAdmin* | Permanently transition state to `READY` and lock setup endpoints |

---

## 3. Authentication & Users

| Method | Path | Permission | Description |
|---|---|---|---|
| `POST` | `/api/v1/auth/login` | Public | Email/username + password login (rate-limited, lockout protected) |
| `POST` | `/api/v1/auth/logout` | Authenticated | Revoke current session and clear cookie |
| `GET` | `/api/v1/auth/me` | Authenticated | Current user profile, role, effective permissions, and CSRF token |
| `GET` | `/api/v1/users` | `users:read` | List users |
| `POST` | `/api/v1/users` | `users:write` | Create user with role |
| `PATCH` | `/api/v1/users/:id` | `users:write` | Update user profile, role, active state |
| `POST` | `/api/v1/users/:id/reset-password` | `users:write` | Reset password (revokes active sessions) |
| `DELETE` | `/api/v1/users/:id` | `users:delete` | Delete user (cannot delete last active `super_admin`) |
| `GET/POST/DELETE` | `/api/v1/auth/tokens` | Authenticated | Manage personal API tokens (plaintext shown once at creation) |

---

## 4. Dashboard & Multi-Server Management

| Method | Path | Permission | Description |
|---|---|---|---|
| `GET` | `/api/v1/dashboard/summary` | `dashboard:read` | Aggregated server counts, container counts, live/latest host metrics (`null` if unavailable), recent events, active alerts |
| `GET` | `/api/v1/servers` | `servers:read` | List all local & remote servers with status, OS, Docker version, capacity |
| `POST` | `/api/v1/servers` | `servers:write` | Register a local or remote server |
| `POST` | `/api/v1/servers/:id/test` | `servers:write` | Test server/agent connectivity |
| `POST` | `/api/v1/servers/:id/registration-token` | `servers:write` | Issue fresh time-limited agent token |
| `DELETE` | `/api/v1/servers/:id` | `servers:delete` | Remove server and revoke agent credentials |

---

## 5. Docker Management (`/api/v1/servers/:server_id/...`)

### Containers
- `GET /containers` (`docker:read`) — Filterable, sortable list with health & port summaries.
- `GET /containers/:cid` (`docker:read`) — Full inspect with **automatic secret masking** on env vars.
- `POST /containers/:cid/env/reveal` (`docker:reveal_secrets`) — Audited reveal of masked env vars.
- `POST /containers/:cid/start|stop|restart|pause|unpause` (`docker:operate`)
- `DELETE /containers/:cid` (`docker:delete`) — Supports `?force=true&v=false`, requires explicit confirmation.
- `GET /containers/:cid/logs` (`logs:read`) — Bounded tail/since snapshot or download (`?download=true`).

### Images, Volumes, Networks, Stacks
- `GET /images`, `POST /images/pull`, `GET /images/:iid`, `DELETE /images/:iid`
- `GET /volumes`, `POST /volumes`, `GET /volumes/:name`, `DELETE /volumes/:name`
- `GET /networks`, `POST /networks`, `GET /networks/:nid`, `DELETE /networks/:nid`
- `GET /stacks`, `GET /stacks/:name`, `POST /stacks/:name/start|stop|restart`, `POST /stacks/deploy`

---

## 6. Infrastructure Monitoring

- `GET /api/v1/servers/:server_id/metrics/host?range=1h` (`monitoring:read`) — Historical host CPU, RAM, disk, disk I/O, network RX/TX, uptime.
- `GET /api/v1/servers/:server_id/containers/:cid/metrics?range=1h` (`monitoring:read`) — Historical container CPU, memory, network, restarts.
- `GET /metrics` (configurable auth via Settings) — Prometheus-compatible text exposition format.

---

## 7. Smart Service Detection, Database Explorer & Redis Explorer

### Discovery
- `GET /api/v1/servers/:server_id/services` (`discovery:read`) — List detected services with confidence (`confirmed`/`probable`/`unknown`) and 5-stage readiness.
- `POST /api/v1/servers/:server_id/services/scan` (`discovery:write`) — Trigger fresh container metadata scan.
- `PUT /api/v1/services/:service_id/credentials` (`services:credentials`) — Save encrypted credentials & test auth.

### Database Explorer (`/api/v1/services/:service_id/db/...`)
- `GET /databases` — List databases / schemas.
- `GET /tables?database=...&schema=...` — List tables, views, or MongoDB collections.
- `GET /tables/:name/schema` — Columns, indexes, constraints, functions.
- `POST /tables/:name/rows/query` (`db:read`) — Parameterized paginated browse/filter (`limit` capped server-side, default `50`, max `500`).
- `POST /tables/:name/rows` | `PATCH` | `DELETE` (`db:write`) — Blocked if connection is in read-only mode; requires explicit confirmation flag.
- `POST /query` (`db:query`) — Execute SQL/Mongo query with timeout, row cap, and read-only enforcement.

### Redis Explorer (`/api/v1/services/:service_id/redis/...`)
- `GET /overview` (`redis:read`) — Parsed `INFO` (memory, keys, clients, ops/sec, hits/misses).
- `GET /keys?cursor=0&pattern=*&count=100` (`redis:read`) — Strictly uses `SCAN`, never `KEYS *`.
- `GET /keys/:key` (`redis:read`) — Type, TTL, memory usage, value (paginated for large collections).
- `PUT /keys/:key`, `POST /keys/:key/rename`, `DELETE /keys/:key` (`redis:write`).
- `POST /command` (`redis:console`) — Guarded command console; destructive commands require `confirm_destructive: true` + `redis:admin`.

---

## 8. Alerts, Audit Logs, Settings & Backup

- `GET/POST/PATCH/DELETE /api/v1/alerts/rules` (`alerts:read` / `alerts:write`)
- `GET /api/v1/alerts/events`, `POST /api/v1/alerts/events/:id/acknowledge|resolve`
- `GET /api/v1/audit-logs` (`audit:read`) — Paginated, filterable immutable audit trail.
- `GET/PATCH /api/v1/settings` (`settings:read` / `settings:write`)
- `POST /api/v1/settings/network/check-port` (`settings:write`) — Real TCP bind probe to check whether a candidate listen port is available (not in use) before saving.
- `POST /api/v1/backup/export`, `POST /api/v1/backup/validate`, `POST /api/v1/backup/restore` (`super_admin` only)

---

## 9. WebSocket Endpoints

All WebSocket endpoints authenticate via session cookie (with `Origin` header validation) or short-lived WS ticket token.

| Path | Permission | Protocol |
|---|---|---|
| `/ws/v1/realtime` | `dashboard:read` | Multiplexed JSON frames: `metrics.host`, `metrics.container`, `docker.event`, `alert.triggered`, `server.status` |
| `/ws/v1/servers/:sid/containers/:cid/logs` | `logs:read` | Server -> client log line frames (`ts`, `stream`, `line`); client -> server control (`pause`, `resume`, `filter`) |
| `/ws/v1/servers/:sid/containers/:cid/exec` | `terminal:exec` | Binary/text PTY frames + JSON resize frame (`{ "type": "resize", "cols": 120, "rows": 36 }`) |
| `/ws/v1/agent/connect` | Agent Auth | Outbound TLS framed RPC + multiplexed stream channel for `meetoops-agent` |
