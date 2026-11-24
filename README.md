# MeetoOps

**Your Infrastructure. One Control Center.**

MeetoOps is a self-hosted, production-oriented web infrastructure management platform inspired by Portainer, Grafana, and Netdata. It combines Docker orchestration, real-time host and container observability, smart service discovery, native database & Redis explorers, centralized log streaming, interactive container terminals, and multi-server remote agent management into a single unified control center.

---

## Core Capabilities

- **Zero-Configuration Installation**: Start with `docker compose up -d`, open the browser, and complete the guided First-Run Setup Wizard — no manual `.env` editing, secret generation, or database migrations required.
- **Docker Management**: Full lifecycle management for containers, images, volumes, networks, and Docker Compose stacks across local and remote hosts.
- **Infrastructure Monitoring**: Real-time and historical CPU, memory, disk, I/O, and network telemetry via Docker Stats API and host collectors, with Prometheus-compatible metrics export.
- **Smart Service Detection**: Automatic detection of MySQL, MariaDB, PostgreSQL, Redis, MongoDB, RabbitMQ, Elasticsearch, Nginx, and Apache with confidence scoring and explicit credential/readiness states.
- **Database Explorer**: Native, parameterized visual explorers for MySQL/MariaDB, PostgreSQL, and MongoDB (read-only by default, guarded mutations, SQL/JSON editors).
- **Redis Explorer**: Key browser using cursor-based `SCAN`, type-aware editors (String, Hash, List, Set, ZSet, Stream, RedisJSON), live telemetry, and permission-guarded command console.
- **Log Center & Terminal**: Bounded real-time log streaming across single or multiple containers, plus authenticated `xterm.js` interactive exec terminals.
- **Multi-Server & Remote Agent**: Lightweight Rust `meetoops-agent` connecting outbound over authenticated TLS — never exposing Docker sockets publicly.
- **Alerts & Audit Logging**: Configurable threshold and health alerts alongside immutable audit logs for security and compliance.
- **Warm Obsidian Design System**: A calm, high-contrast, dark-first technical workstation aesthetic designed for long operational sessions.

---

## Technology Stack

| Layer | Technologies |
|---|---|
| **Frontend** | Svelte 5, TypeScript, Vite, Tailwind CSS, shadcn-svelte, Lucide Icons, TanStack Table & Virtual, Apache ECharts, xterm.js, Monaco Editor |
| **Backend** | Rust, Axum, Tokio, Bollard (Docker Engine API), Serde, SQLx, `redis-rs`, `tracing`, WebSockets |
| **Database** | SQLite (default, WAL mode) with PostgreSQL-ready persistence architecture |
| **Security** | Argon2id password hashing, HttpOnly secure sessions, RBAC, ChaCha20-Poly1305 / AES-256-GCM secret encryption, outbound TLS agent auth |
| **Deployment** | Multi-stage Docker builds, Docker Compose, persistent named volumes |

---

## Quick Start (Target Production Experience)

```bash
mkdir -p meetoops && cd meetoops
curl -fsSL https://get.meetoops.com/docker-compose.yml -o docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8080` to launch the **First-Run Setup Wizard**.

---

## Documentation Index

### Architecture & Engineering
- [AGENTS.md](./AGENTS.md) — Engineering rules, coding standards, and non-negotiable invariants
- [DESIGN.md](./DESIGN.md) — Warm Obsidian quick reference & semantic tokens
- [docs/DESIGN-SYSTEM.md](./docs/DESIGN-SYSTEM.md) — Full UI/UX design system specification
- [docs/IMPLEMENTATION-PLAN.md](./docs/IMPLEMENTATION-PLAN.md) — Phased implementation roadmap and milestones
- [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) — System architecture, crate boundaries, and agent protocol
- [docs/DATABASE.md](./docs/DATABASE.md) — Application database schema, migrations, and persistence design
- [docs/API.md](./docs/API.md) — REST API and WebSocket protocol specification
- [docs/SECURITY.md](./docs/SECURITY.md) — Threat model, authentication, RBAC, encryption, and audit logging

### Installation & Operations
- [docs/INSTALLATION.md](./docs/INSTALLATION.md) — Zero-configuration installation guide
- [docs/FIRST-RUN-SETUP.md](./docs/FIRST-RUN-SETUP.md) — Guided first-run setup wizard & state machine
- [docs/CONFIGURATION.md](./docs/CONFIGURATION.md) — Settings storage, master key lifecycle, and env overrides
- [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) — Production Docker Compose, reverse proxy, and TLS guide
- [docs/SECURITY-DEPLOYMENT.md](./docs/SECURITY-DEPLOYMENT.md) — Deployment hardening and Docker socket risk model
- [docs/UPGRADE.md](./docs/UPGRADE.md) — Safe zero-data-loss upgrades and migration handling
- [docs/BACKUP-RESTORE.md](./docs/BACKUP-RESTORE.md) — Database, config, and encryption key backup & recovery
- [docs/DEVELOPMENT.md](./docs/DEVELOPMENT.md) — Local development setup and testing workflow

---

## Project Status

**Current Stage:** **v0.1.0 — All Roadmap Phases (Phase 0–5) Complete.**
See [docs/IMPLEMENTATION-PLAN.md](./docs/IMPLEMENTATION-PLAN.md) for the full feature breakdown.

---

## Open Source & Contributing

MeetoOps is **100% open source** under the [MIT License](./LICENSE). Everyone is free to self-host, use, modify, and contribute to the project.

- Read [CONTRIBUTING.md](./CONTRIBUTING.md) to get started with local development and pull requests.
- Follow our [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
- Report security issues responsibly via [SECURITY.md](./SECURITY.md).
