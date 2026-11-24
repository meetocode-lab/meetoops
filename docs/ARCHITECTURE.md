# MeetoOps — System Architecture

## 1. High-Level Topology

MeetoOps follows a **Centralized Control Plane + Outbound Remote Agent** architecture designed for self-hosted, multi-server Docker and infrastructure operations.

```text
                       +-----------------------------------+
                       |         MeetoOps Web (SPA)        |
                       |  Svelte 5 / TS / Tailwind / WS    |
                       +-----------------+-----------------+
                                         | HTTPS / WSS (REST + Realtime)
                                         v
+------------------------------------------------------------------------------+
|                         MeetoOps Server (Rust / Axum)                        |
|                                                                              |
|  +--------------+  +---------------+  +---------------+  +----------------+  |
|  | Auth & RBAC  |  | Setup & Config|  | Docker Router |  | Alert & Audit  |  |
|  +--------------+  +---------------+  +-------+-------+  +----------------+  |
|                                               |                              |
|  +--------------+  +---------------+          |          +----------------+  |
|  | DB Explorers |  | Redis Explorer|          |          | Monitoring Hub |  |
|  | (PG/My/Mongo)|  | (redis-rs)    |          |          | (Metrics+Prom) |  |
|  +--------------+  +---------------+          |          +----------------+  |
|                                               |                              |
|  +---------------------------------+  +-------+---------------------------+  |
|  | SQLite DB (WAL) + Encrypted     |  | Agent Tunnel Gateway (WSS/TLS)    |  |
|  | Master Key Volume               |  | Outbound Authenticated Agents     |  |
|  +---------------------------------+  +-------+---------------------------+  |
+-----------------------------------------------|------------------------------+
             |                                  ^
             | Local Socket (Optional)          | Outbound Authenticated TLS
             v                                  |
   +--------------------+            +----------+-----------+
   | Local Docker Host  |            | Remote Host (1..N)   |
   | /var/run/docker.sock|           |  +----------------+  |
   +--------------------+            |  | MeetoOps Agent |  |
                                     |  | (Rust Binary)  |  |
                                     |  +-------+--------+  |
                                     |          |           |
                                     |  Local Docker Socket |
                                     |  + Host /proc Stats  |
                                     +----------------------+
```

---

## 2. Core Components

### 2.1 MeetoOps Web (`apps/web`)
- Single-Page Application built with **Svelte 5**, **TypeScript (strict)**, **Vite**, **Tailwind CSS**, and **shadcn-svelte**.
- In production, compiled static assets are embedded/served directly by `meetoops-server` (or fronted by a reverse proxy) so a single container exposes port `8080` with zero CORS friction.
- Connects via REST (`/api/v1/*`) for queries/mutations and WebSockets (`/ws/v1/*`) for live dashboard telemetry, log streaming, container events, and `xterm.js` terminal sessions.

### 2.2 MeetoOps Server (`services/server`)
- Single statically-linked Rust binary powered by **Tokio** and **Axum**.
- **Responsibilities**:
  - Automatic zero-config bootstrap (SQLite creation, embedded SQLx migrations, persistent master encryption key initialization).
  - First-Run Setup Wizard state machine (`UNINITIALIZED` -> `READY`).
  - Session & API token authentication, RBAC enforcement, CSRF & rate limiting.
  - Unified `ServerTransport` abstraction that routes Docker, discovery, monitoring, log, and terminal commands identically to either the **Local Docker Engine** or a **Connected Remote Agent**.
  - Native database explorer connection pools (PostgreSQL, MySQL/MariaDB, MongoDB) and Redis explorer (`redis-rs`).
  - Real-time metrics aggregation, historical SQLite rollups, Prometheus exporter, alert evaluation loop, and immutable audit logging.

### 2.3 MeetoOps Agent (`services/agent`)
- Lightweight, single-binary Rust daemon deployed on remote Docker hosts.
- **Outbound-Only TLS Connection**: Connects *outward* to MeetoOps Server over `wss://` (TLS 1.3). Remote hosts never open inbound ports or expose Docker TCP port `2375`.
- **Registration & Auth**:
  1. Admin generates a single-use, time-limited (15 min) registration token in MeetoOps Server.
  2. Agent boots with the server URL and registration token, exchanges it over TLS for a unique `agent_id` + long-lived high-entropy `agent_secret` (persisted locally on the agent host).
  3. Subsequent connections authenticate via signed bearer/HMAC challenge or mTLS.
- **Capabilities**:
  - Local Docker operations via Bollard.
  - Host & container metric sampling pushed on configured intervals.
  - Multiplexed streaming channels for container logs and interactive `exec` TTY sessions.
  - Automatic exponential-backoff reconnect with heartbeat monitoring.

---

## 3. Rust Workspace Crate Boundaries

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `crates/common` | Shared DTOs, error enums, pagination, redaction masks, protocol frames | `serde`, `thiserror`, `chrono`, `uuid` |
| `crates/security` | Argon2id password hashing, session/token crypto, AEAD (`AES-256-GCM`) vault, master key loader, RBAC evaluator, rate limiter | `argon2`, `aes-gcm`, `rand`, `zeroize`, `sha2` |
| `crates/core` | SQLx persistence layer, migrations, setup state machine, settings service, user/role service, audit logger, alert engine | `sqlx`, `tokio`, `tracing`, `common`, `security` |
| `crates/docker` | Bollard client wrapper: containers, images, volumes, networks, Compose stack detection/deploy, logs, exec PTY | `bollard`, `futures-util`, `common`, `security` |
| `crates/discovery` | Rule-based service detector (image, tags, labels, ports, env keys, network) with confidence & 5-stage readiness | `common`, `docker` |
| `crates/monitoring` | Host collector (`sysinfo`), container stats aggregator, ring buffers, SQLite rollups, Prometheus text renderer | `sysinfo`, `bollard`, `common`, `core` |
| `crates/database` | Native async MySQL/MariaDB, PostgreSQL, and MongoDB explorer engines with parameterized queries & pool limits | `sqlx`, `mongodb`, `common`, `security` |
| `crates/redis` | `redis-rs` explorer: `INFO` parser, cursor `SCAN`, type editors, guarded command execution | `redis`, `common`, `security` |

---

## 4. Key Architectural Flows

### 4.1 First-Run Initialization State Machine

```text
UNINITIALIZED
      |  (server boots, DB migrated, master key verified)
      v
FIRST_RUN_SETUP
      |  (POST /api/v1/setup/admin — atomic DB transaction)
      v
ADMIN_CREATED
      |  (optional preferences + local/remote Docker step)
      v
INFRASTRUCTURE_SETUP
      |  (POST /api/v1/setup/complete — irreversible lock)
      v
    READY  ---> (All /api/v1/setup/* write endpoints permanently return 403)
```

### 4.2 Smart Service Detection 5-Stage Model

Service discovery strictly separates **detection** from **access**:

1. `detected` — Container matches heuristics for a known service (e.g., `postgres:16-alpine` -> PostgreSQL, confidence `Confirmed`).
2. `connection_discovered` — Host/container network address and port identified (e.g., internal bridge IP `172.18.0.4:5432` or published port).
3. `credentials_available` — User has explicitly supplied and saved encrypted credentials in MeetoOps (never scraped silently into unverified auto-login).
4. `auth_verified` — Live driver handshake (`SELECT 1` / `PING`) succeeded.
5. `explorer_ready` — Visual Database or Redis Explorer unlocked for authorized users.

### 4.3 Real-Time Data & Backpressure

- Server maintains a `RealtimeHub` using Tokio `broadcast` channels partitioned by topic (`dashboard`, `server:<id>:metrics`, `alerts`).
- Per-stream operations (container logs, exec terminal) use dedicated bounded `mpsc` channels (`capacity = 256` frames) tied to a `CancellationToken`.
- Slow WebSocket clients drop lagged metric snapshots cleanly rather than growing server memory unboundedly.

---

## 5. Architectural Decision Records (Summary)

- **ADR-01: SQLite (WAL) Default + Postgres-Ready Traits** — Guarantees zero-config single-container deployment while repository traits isolate dialect differences for future enterprise PostgreSQL support.
- **ADR-02: Single Binary + Embedded Web Assets** — Eliminates CORS misconfigurations and simplifies `docker compose up -d` to a single exposed HTTP port (`8080`).
- **ADR-03: Outbound TLS Agent Protocol** — Eliminates the security hazard of exposing Docker TCP `2375` or opening inbound firewall rules on managed hosts.
- **ADR-04: Explicit `Option<f64>` (`null`) for Telemetry** — Prevents misleading `0%` charts when a collector or agent is disconnected or a metric is unsupported on the host OS.
