# MeetoOps — Local Development Guide

## 1. Prerequisites

- **Rust**: stable toolchain (`rustc`, `cargo`)
- **Node.js**: v20+ (`npm` or `bun`)
- **Docker Engine** *(optional for Phase 0 UI/API work; required for live Docker container/monitoring/DB tests)*

---

## 2. Repository Structure

```text
meetoops/
├── apps/web/              # Svelte 5 + TypeScript + Vite + Tailwind + shadcn-svelte
├── services/server/       # Axum API + WS server binary
├── services/agent/        # Remote host agent binary
├── crates/                # Modular Rust libraries (common, security, core, docker, discovery, monitoring, database, redis)
├── migrations/            # Versioned SQLx SQL migrations
├── deployments/           # Docker Compose & Dockerfiles
└── docs/                  # Architecture, design, security, and operations docs
```

---

## 3. Running Locally (Dev Mode)

### Backend (`meetoops-server`)
No `.env` file is required. In dev mode, MeetoOps stores its SQLite database and auto-generated master key in `./.meetoops-dev/` by default (gitignored):

```bash
cargo run -p meetoops-server
# Listens on http://127.0.0.1:9480
```

### Frontend (`apps/web`)
```bash
cd apps/web
bun install
bun run dev
# Vite dev server runs on http://127.0.0.1:5173 and proxies /api and /ws to 127.0.0.1:9480
```

---

## 4. Testing & Quality Checks

### Backend
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Frontend
```bash
cd apps/web
bun run check
bun run build
```

---

## 5. Resetting Local Dev State

To re-test the **First-Run Setup Wizard** from scratch in local development, stop `meetoops-server` and delete `./.meetoops-dev/`, then start `meetoops-server` again.
