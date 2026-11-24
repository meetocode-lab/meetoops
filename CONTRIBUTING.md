# Contributing to MeetoOps

Thank you for your interest in contributing to **MeetoOps**! MeetoOps is open source — everyone is welcome to use it, report issues, suggest improvements, and submit pull requests.

---

## Ways to Contribute

- **Report Bugs**: Open an issue with reproduction steps, environment details (OS, Docker version, MeetoOps version), and sanitized logs (never include passwords, tokens, or secrets).
- **Suggest Features**: Propose new capabilities or UX improvements aligned with MeetoOps's focus on real, production-grade self-hosted infrastructure management.
- **Submit Code & Docs**: Fix bugs, implement roadmap modules, improve test coverage, or refine documentation.

---

## Before You Start

Please read these foundational documents so your contribution fits seamlessly into the codebase:
1. [AGENTS.md](./AGENTS.md) — Engineering standards, zero-fake-data rule, crate boundaries, and security checklist.
2. [DESIGN.md](./DESIGN.md) & [docs/DESIGN-SYSTEM.md](./docs/DESIGN-SYSTEM.md) — Warm Obsidian semantic tokens and UI conventions.
3. [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) — Backend & remote agent architecture.
4. [docs/DEVELOPMENT.md](./docs/DEVELOPMENT.md) — Local development setup.

---

## Pull Request Workflow

1. **Fork & Branch**: Create a feature or fix branch from `main` (e.g., `feat/redis-stream-editor` or `fix/docker-log-reconnect`).
2. **Keep Changes Focused**: One logical feature or bug fix per PR.
3. **Follow Engineering Rules**:
   - No fake metrics, mock operational buttons, or hardcoded raw hex colors in UI components.
   - All mutating endpoints must enforce server-side RBAC and emit audit log entries.
   - Schema changes must be forward-only numbered SQL migrations in `migrations/`.
4. **Verify Locally Before Opening a PR**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   ```bash
   cd apps/web
   npm run check
   npm run build
   ```
5. **Open a Pull Request**: Describe what changed, why, how you tested it, and attach screenshots for UI changes.

---

## Community Standards

All participants are expected to follow our [Code of Conduct](./CODE_OF_CONDUCT.md). By contributing, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
