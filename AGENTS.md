# AGENTS.md — MeetoOps Engineering Rules & Standards

This document defines the mandatory engineering rules, architectural boundaries, security invariants, and workflow conventions for all human engineers and AI coding agents contributing to **MeetoOps**.

---

## 1. Product Philosophy & Non-Negotiable Principles

1. **Real Infrastructure Only — Zero Fake Data**
   - Every UI control, metric chart, container action, database query, Redis command, log stream, and terminal session MUST be backed by real backend operations.
   - Never hardcode fake metrics, mock container lists, placeholder latency numbers, or simulated success responses in production code paths.
   - Never represent missing/unavailable metrics as `0`. Use explicit `null` / `unavailable` states in both API payloads and UI components.
   - If a feature or module is not yet implemented in the current phase, clearly mark it as **Unavailable / Coming in Phase X** rather than faking results.

2. **Zero-Configuration First-Run Experience**
   - A new user must be able to run `docker compose up -d`, open a browser, complete the First-Run Setup Wizard, and immediately manage infrastructure without manually creating `.env` files, running SQL migrations, or generating secrets.
   - All runtime settings live in the application database; secrets live in authenticated encrypted storage backed by a persistent auto-generated master key volume.
   - Environment variables exist strictly for deployment-level overrides, never as a prerequisite for first-time installation.

3. **Security by Default (Least Privilege)**
   - Never expose an unauthenticated Docker socket or Docker TCP port (`2375`).
   - Never store credentials, API tokens, or database passwords in plaintext.
   - Never log passwords, session tokens, encryption keys, or secret environment variables.
   - Never trust client-side role/permission checks; every API route and WebSocket handler MUST enforce server-side authentication and RBAC authorization.
   - Never execute SQL via shell commands or string concatenation; always use official drivers with parameterized queries.
   - Never run `KEYS *` by default against Redis; always use cursor-based `SCAN`.

4. **Warm Obsidian Visual Identity**
   - Follow [DESIGN.md](./DESIGN.md) and [docs/DESIGN-SYSTEM.md](./docs/DESIGN-SYSTEM.md) strictly.
   - Use semantic CSS tokens (`--background`, `--surface`, `--primary`, etc.) exclusively. Never scatter raw hex codes in components.
   - No neon blue/purple DevOps clichés, excessive glassmorphism, gratuitous gradients, or decorative filler cards.

---

## 2. Repository & Architecture Rules

### Workspace Layout

```text
meetoops/
├── apps/
│   └── web/               # Svelte 5 + TypeScript + Vite + Tailwind + shadcn-svelte
├── services/
│   ├── server/            # MeetoOps Server binary (Axum REST/WS, orchestration)
│   └── agent/             # MeetoOps Agent binary (outbound TLS remote host agent)
├── crates/
│   ├── common/            # Shared types, errors, DTOs, pagination, redaction
│   ├── core/              # Config, app state machine, settings, RBAC, audit, alerts
│   ├── security/          # Argon2id, sessions, CSRF, AEAD encryption, rate limiting
│   ├── docker/            # Bollard adapter: containers, images, volumes, networks, compose, exec
│   ├── discovery/         # Smart service detection (confidence + 5-stage readiness)
│   ├── monitoring/        # Host & container collectors, ring buffers, SQLite rollups, Prom export
│   ├── database/          # MySQL/MariaDB, PostgreSQL, MongoDB native explorer adapters
│   └── redis/             # redis-rs explorer, SCAN browser, type editors, guarded console
├── deployments/           # Production & dev Docker Compose, Dockerfiles, reverse proxy examples
├── migrations/            # Versioned SQLx migrations (SQLite default, Postgres-compatible SQL)
└── docs/                  # Architecture, API, Database, Security, Deployment, Ops guides
```

### Module Dependency Rules

- `crates/common` has **zero** internal crate dependencies.
- `crates/security` depends only on `crates/common`.
- Domain crates (`docker`, `discovery`, `monitoring`, `database`, `redis`) depend on `common` (and `security` where needed for secret handling/redaction), never on `services/server`.
- `crates/core` orchestrates persistence (SQLx), setup state machine, RBAC, audit logging, and alerting.
- `services/server` wires HTTP/WebSocket handlers, middleware, and background workers.
- `services/agent` depends only on `common`, `security`, `docker`, `discovery`, and `monitoring` — never on `services/server` or database explorer crates.

---

## 3. Backend Engineering Standards (Rust)

- **Async Runtime**: Tokio (`multi_thread`). Never block async worker threads with synchronous I/O or heavy CPU work (use `tokio::task::spawn_blocking` for Argon2id hashing or CPU-intensive compression/parsing).
- **Error Handling**:
  - Use `thiserror` in library crates (`crates/*`) with structured domain error variants.
  - Map domain errors to consistent JSON API error envelopes (`{ code, message, details?, request_id }`) in `services/server`.
  - Never leak internal stack traces, SQL errors, or raw driver credentials in HTTP responses.
- **Database Persistence (SQLx)**:
  - Default engine: SQLite (`WAL` mode, `foreign_keys = ON`, `busy_timeout = 5000`).
  - Write ANSI-standard SQL where possible and isolate dialect-specific queries behind repository traits so PostgreSQL support can be added without rewriting business logic.
  - Every schema change MUST be a forward-only numbered migration in `migrations/`. Never mutate an already-applied migration.
- **Streaming & Resource Bounds**:
  - All log streams, metric channels, and terminal pipes MUST use bounded channels (`tokio::sync::mpsc::channel(N)` or `broadcast::channel(N)`).
  - Support graceful cancellation (`CancellationToken`) on client disconnect or server shutdown.
- **Observability**:
  - Use `tracing` with structured fields (`request_id`, `user_id`, `server_id`, `container_id`).
  - Never pass secrets into `tracing` macros; wrap sensitive structs with custom `Debug` implementations that print `[REDACTED]`.

---

## 4. Frontend Engineering Standards (Svelte 5 + TypeScript)

- **Strict TypeScript**: `strict: true`, no `any` in application code. Define typed API clients matching backend DTOs.
- **Svelte 5 Runes**: Use `$state`, `$derived`, `$effect`, `$props` consistently. Avoid mixing legacy Svelte 4 reactive syntax (`$:`) with Svelte 5 runes.
- **Component Architecture**:
  - Build reusable primitives in `apps/web/src/components/ui/` (shadcn-style Blue Aurora components) and domain components in `apps/web/src/components/`.
  - **NEVER use raw browser-default `<select>`, `<input type="checkbox">`, or native `<input type="date">`** in application views. Always use the shared UI primitives (`Select.svelte`, `Checkbox.svelte`, `DatePicker.svelte`) so styling is 100% consistent across Light and Dark modes.
  - **`Select.svelte` Rule**: Searchable combobox behavior MUST be enabled by default (`searchable = true`), and may only be disabled (`searchable={false}`) for small fixed option sets (e.g., 2–4 static items).
  - **Independent Scroll Shell**: The authenticated app shell MUST lock viewport height (`h-screen overflow-hidden`) so the Sidebar and Top Navbar stay fixed while the Sidebar `<nav>` and `<main>` workspace each scroll independently.
  - **Full-Width Workspace**: Workspace views inside `<main>` must use full width (`w-full`) without arbitrary `max-w-*` containers leaving empty margins on desktop screens.
  - **Server-Side Data Tables**: Every table and list MUST default to an inline toolbar containing **Search** + contextual **Custom Filters** (`Select`, `MultiSelect`, `DatePicker`, `DateRangePicker`), **per-column sorting** (`sort_by`, `sort_dir`), and **server-side pagination with selectable page size** (`10 / 25 / 50 / 100`) using `PaginationBar.svelte`. Never load unbounded full tables into the browser to filter/paginate client-side.
  - **Skeleton Loading Only**: NEVER render plain `"Loading…"` text or bare centered spinners for initial page, card, chart, or table loading states. Always use `Skeleton.svelte` and `TableSkeleton.svelte` matching the target layout to prevent layout shift across Light and Dark modes.
  - Every data view MUST handle four explicit states: `loading` (with Skeleton), `empty`, `error`, and `ready`.
  - All destructive actions (container remove, image remove, volume prune, DB row delete, Redis key delete, user delete) MUST require explicit user confirmation showing the target resource name.
- **Performance**:
  - Use `@tanstack/svelte-table` + `@tanstack/svelte-virtual` for container lists, log lines, database result grids, and Redis key lists.
  - Use WebSockets for real-time updates (dashboard metrics, container events, log streams, terminal I/O). Do not set aggressive `setInterval` polling loops when a WebSocket stream is available.
  - Cap client-side log ring buffers (e.g., max 5,000 lines in memory) to prevent browser OOM.

---

## 5. Security Checklist (Required for Every PR / Phase)

Before completing any feature or phase, verify:

- [ ] All endpoints (except `/api/v1/health`, `/api/v1/setup/status`, `/api/v1/setup/*` when uninitialized, and `/api/v1/auth/login`) require a valid authenticated session or API token.
- [ ] Setup endpoints return `403 Forbidden` / `404` once initialization state passes `ADMIN_CREATED` / `READY`.
- [ ] RBAC permission check is enforced server-side on every mutating and sensitive read endpoint.
- [ ] All sensitive mutations emit an immutable record to `audit_logs`.
- [ ] Secret env vars (`*PASSWORD*`, `*SECRET*`, `*TOKEN*`, `*KEY*`, `*DSN*`, `*PRIVATE*`, `*CREDENTIAL*`) are masked in container inspection responses unless explicitly revealed by an authorized role (and that reveal is audited).
- [ ] All user inputs (container names, volume names, SQL pagination limits, Redis patterns, hostnames) are validated server-side.
- [ ] Rate limiting is active on login, setup, and token endpoints.

---

## 6. Testing & Verification Rules

- **Buildability**: The entire workspace (`cargo check --workspace`, `cargo test --workspace`, and `npm run build` / `npm run check` in `apps/web`) MUST succeed after every phase.
- **Unit & Integration Tests**:
  - Test security primitives (Argon2id hashing, AEAD encrypt/decrypt, key tamper detection, secret masking, RBAC matrix, setup state lock).
  - Test service discovery heuristics against representative container metadata fixtures.
  - Test SQLx migrations against a fresh SQLite database on startup.
- **Incremental Delivery**:
  - Inspect existing code before modifying anything.
  - Never overwrite existing work without understanding its purpose.
  - Follow the phased roadmap in [docs/IMPLEMENTATION-PLAN.md](./docs/IMPLEMENTATION-PLAN.md).
