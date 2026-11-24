# MeetoOps — Implementation Plan & Phased Roadmap

This document translates the MeetoOps Master Specification into a disciplined, verifiable, phase-by-phase engineering plan. Every phase ends with a buildable, runnable, and tested application.

---

## Phase 0: Foundation, Zero-Config Bootstrap & Design System

**Goal:** Establish the Rust workspace, Svelte 5 frontend, Warm Obsidian design system, SQLite persistence with auto-migrations, auto-generated encryption key management, First-Run Setup Wizard, authentication, and initial live Dashboard shell.

### Deliverables
1. **Rust Workspace (`Cargo.toml` + `crates/*` + `services/server` + `services/agent`)**
   - `crates/common`: API response envelopes, domain error types, pagination structs, secret redaction helpers.
   - `crates/security`: Argon2id hashing, secure session tokens, CSRF token generation/verification, AEAD (`AES-256-GCM`) secret vault with persistent master key file loader/generator, rate limiter.
   - `crates/core`: SQLx SQLite pool (`WAL`, `foreign_keys`), embedded auto-migrations, First-Run Setup state machine (`UNINITIALIZED` -> `FIRST_RUN_SETUP` -> `ADMIN_CREATED` -> `INFRASTRUCTURE_SETUP` -> `READY`), settings repository, user & RBAC repository, audit log writer.
   - `services/server`: Axum HTTP server, static frontend serving (for single-container / production deployment), `/api/v1/health`, `/api/v1/setup/*`, `/api/v1/auth/*`, `/api/v1/settings/*`, `/api/v1/users/*`, `/api/v1/audit-logs`, and WebSocket hub foundation.
2. **Svelte 5 Frontend (`apps/web`)**
   - Vite + Svelte 5 + TypeScript (strict) + Tailwind CSS + shadcn-svelte + Lucide Icons.
   - Full Warm Obsidian CSS token architecture (dark default, light-warm ready).
   - App shell: collapsible Dark Espresso sidebar, top status/context bar, server switcher, toast notifications, confirmation modal system.
   - **First-Run Setup Wizard** (5 steps: Welcome, Create Super Admin, Preferences, Connect Local/Remote Docker, Complete) wired to real `/api/v1/setup/*` endpoints.
   - **Login page** + session guard + automatic redirect based on `/api/v1/setup/status`.
   - **Dashboard shell** connected to real backend health/system status APIs (no fake metrics — unavailable modules explicitly labeled).
   - **Settings UI** (General, Users & Roles, Security, Audit Logs).
3. **Docker & Dev Tooling**
   - Multi-stage `Dockerfile` (server + web, and agent).
   - Zero-config `deployments/docker-compose.yml` (no `.env` required).
   - Unit tests for security vault, Argon2id, setup lock atomicity, and DB migrations.

**Exit Criteria:**
- `cargo test --workspace` passes.
- `npm run check && npm run build` passes.
- Starting `meetoops-server` against an empty data directory auto-creates SQLite DB, runs migrations, generates master encryption key, serves the First-Run Setup Wizard, creates Super Admin, locks setup permanently, and loads the authenticated Dashboard.

---

## Phase 1: Core Docker Management & Log Streaming

**Goal:** Deliver real Docker Engine operations via Bollard and real-time bounded log streaming.

### Deliverables
1. **`crates/docker`**
   - Local Docker socket / named pipe (`//./pipe/docker_engine` on Windows, `/var/run/docker.sock` on Linux) connection manager with explicit availability detection and safe error reporting.
   - Container operations: list, inspect (with automatic env secret masking), start, stop, restart, pause, unpause, remove.
   - Image operations: list, inspect, pull (streaming progress), remove, layers/metadata.
   - Volume operations: list, inspect, create, remove, mounted container cross-reference.
   - Network operations: list, inspect, create, remove, connected container list.
   - Log streaming: Bollard `logs` stream over bounded Tokio channel -> WebSocket.
2. **Backend API & WS**
   - `/api/v1/servers/:id/containers/*`, `/images/*`, `/volumes/*`, `/networks/*`.
   - `/ws/v1/servers/:id/containers/:cid/logs` (streaming, tail, timestamps, follow, pause/resume).
   - Audit logging for all lifecycle mutations and secret reveals.
3. **Frontend UI**
   - Containers table (TanStack Table + Virtual), filters, status badges, bulk-safe actions, detail drawer/page (Overview, Env with masked secrets, Ports, Mounts, Networks, Labels, Live Logs).
   - Images, Volumes, Networks management views.
   - Log Center (single & multi-container streaming, search, pause/resume, follow, download).

**Exit Criteria:**
- Real containers on a connected Docker host can be inspected, started, stopped, restarted, paused, removed, and streamed live in the Log Center.

---

## Phase 2: Infrastructure Monitoring & Real-Time Dashboard

**Goal:** Deliver real host and container telemetry, historical rollups, ECharts visualizations, and a live WebSocket-driven Dashboard.

### Deliverables
1. **`crates/monitoring`**
   - Host collector (`sysinfo` / `/proc` on Linux): CPU %, RAM usage, Disk usage & I/O, Network RX/TX, system uptime.
   - Container collector: Bollard stats stream (CPU %, memory usage/limit, network RX/TX, restart count, health status).
   - Configurable collection intervals (stored in DB settings).
   - Historical storage & downsampling rollups in SQLite (`metrics_host`, `metrics_container`) with retention pruning.
   - Prometheus `/metrics` endpoint (authenticated or toggleable via Settings).
   - Explicit `null` handling for unavailable metrics (never coerced to `0`).
2. **Frontend UI**
   - Real-time Dashboard with live KPI strips, server/container counts, Apache ECharts time-series (Warm Obsidian palette: Muted Amber, Terracotta, Sage, Muted Blue), recent events, and active alerts.
   - Per-server and per-container Monitoring views with time-range selector (`15m`, `1h`, `6h`, `24h`, `7d`).

**Exit Criteria:**
- Dashboard and Monitoring charts update in real time via WebSocket with real host/container metrics and display historical queries accurately.

---

## Phase 3: Smart Service Detection, Database Explorer & Redis Explorer

**Goal:** Automatically detect running services inside containers and provide secure, native database and Redis explorers.

### Deliverables
1. **`crates/discovery`**
   - Heuristics inspecting image name/tag, labels, exposed ports, non-secret env keys, and network config for: MySQL, MariaDB, PostgreSQL, Redis, MongoDB, RabbitMQ, Elasticsearch, Nginx, Apache.
   - Confidence scoring (`Confirmed`, `Probable`, `Unknown`) and 5-stage readiness state (`detected` -> `connection_discovered` -> `credentials_available` -> `auth_verified` -> `explorer_ready`).
   - Encrypted credential attachment flow (never auto-assuming access).
2. **`crates/database`**
   - Native async drivers: SQLx MySQL/MariaDB, SQLx PostgreSQL, official `mongodb` Rust driver.
   - Connection pooling with idle eviction, strict timeouts, parameterized queries, enforced row limits, read-only default mode, and explicit confirmation for DML/DDL mutations.
   - Schema/table/collection browsers, column/index/constraint inspectors, record viewer with pagination/filtering, Monaco SQL/JSON editor, CSV/JSON export.
3. **`crates/redis`**
   - `redis-rs` async connection manager.
   - `INFO` telemetry parser (memory, keys, clients, ops/sec, hit/miss ratio).
   - Cursor-based `SCAN` key browser (never `KEYS *` by default), TTL & type inspection, type-aware editors (String, Hash, List, Set, ZSet, Stream, RedisJSON).
   - Guarded command console with RBAC checks and destructive command confirmation (`FLUSHALL`, `FLUSHDB`, `CONFIG`, `SHUTDOWN`, `DEL`).

**Exit Criteria:**
- Detected Postgres/MySQL/MongoDB/Redis containers appear in Service Discovery, accept encrypted user credentials, and open fully functional visual explorers.

---

## Phase 4: Terminal, Compose Stacks, Multi-Server Remote Agent & Alerts

**Goal:** Complete advanced operational features for multi-host production environments.

### Deliverables
1. **Interactive Container Terminal**
   - Bollard `create_exec` + `start_exec` (TTY, stdin/stdout/stderr, resize) bridged over authenticated WebSocket to `xterm.js` (Warm Obsidian terminal theme).
   - Shell auto-detection (`/bin/bash`, `/bin/sh`, `ash`), session timeout, RBAC check (`terminal:exec`), and audit logging.
2. **Docker Compose / Stacks**
   - Detect Compose projects via `com.docker.compose.project` labels.
   - Inspect stack services/status, start/stop/restart stacks, and deploy Compose YAML via Monaco Editor.
3. **MeetoOps Agent (`services/agent`) & Multi-Server Management**
   - Outbound authenticated TLS WebSocket/gRPC-style framed control channel from Agent to Server.
   - Time-limited single-use registration tokens -> issued long-lived rotated agent credentials.
   - Agent relays host metrics, container stats, Docker operations, log streams, and exec sessions.
   - Server grouping, health heartbeat, disconnect detection, and Setup Wizard / Settings onboarding commands.
4. **Alerts & Notifications**
   - Rule engine evaluating CPU, RAM, Disk, container stopped/unhealthy, restart loops, agent disconnect, and DB connection failures.
   - Severity levels (`info`, `warning`, `critical`), alert history, acknowledge & resolve actions, and webhook/notification hooks.

**Exit Criteria:**
- A remote host running `meetoops-agent` registers via token, connects outbound over TLS, appears in Multi-Server view, and supports full Docker/monitoring/logs/terminal operations plus alerting.

---

## Phase 5: Production Hardening, Backup/Restore & Release Readiness

**Goal:** Harden security, optimize performance under load, implement backup/restore, and finalize production packaging.

### Deliverables
1. **Backup & Restore**: Web UI + API for encrypted configuration/database export, integrity checksum validation, and safe restore flow.
2. **Security Hardening**: Security headers, strict CSP, session revocation, API token scopes, brute-force lockouts, penetration checklist verification.
3. **Performance & Load Verification**: Virtualized table stress tests (1,000+ containers / 100k log lines), bounded memory profiling, DB vacuum/retention tuning.
4. **End-to-End & Integration Tests**: Automated Docker Compose smoke tests verifying zero-config boot, setup wizard, persistence across `docker compose down && docker compose up -d`, and upgrade safety.
