# MeetoOps — Security Architecture & Threat Model

Security in MeetoOps is a foundational invariant, not an add-on. Because MeetoOps operates Docker engines, databases, Redis clusters, and interactive container terminals, compromise of the control plane could escalate to host compromise. Every subsystem is engineered under **defense in depth** and **least privilege**.

---

## 1. Threat Model & Core Defenses

| Threat Vector | Primary Mitigation |
|---|---|
| First-run takeover / race condition | Atomic SQLite transaction on singleton `app_state` row; permanent lock once `ADMIN_CREATED`/`READY`; rate limiting on `/setup/*` |
| Credential theft / DB leak | Argon2id password hashing; `AES-256-GCM` authenticated encryption for all stored secrets; master key stored outside DB in dedicated volume (`0600`) |
| Unauthorized Docker / Terminal access | Server-side RBAC on every REST & WS endpoint; `Origin` validation on WS upgrades; audited exec sessions; never expose Docker TCP `2375` |
| Remote host compromise via inbound ports | `meetoops-agent` uses outbound-only authenticated TLS (`wss://`); time-limited single-use registration tokens -> rotated agent secrets |
| Secret leakage in UI or logs | Automatic regex/key-based env masking; custom `Debug` redaction in Rust structs; audit logs strip sensitive payloads |
| SQL / Redis injection & destructive ops | Official parameterized DB drivers (zero shell execution); read-only default mode; cursor `SCAN` (no `KEYS *`); explicit destructive confirmation + RBAC |
| Session hijacking & CSRF | `HttpOnly`, `SameSite=Lax`, `Secure` (under HTTPS) opaque random 256-bit session tokens (SHA-256 hashed in DB); per-session `X-CSRF-Token` |
| Brute-force login / API abuse | Per-IP and per-account token-bucket rate limiting + progressive account lockout (`failed_login_attempts` / `locked_until`) |

---

## 2. Authentication & Session Security

### Password Hashing
- Algorithm: **Argon2id** (`m=19456 KiB`, `t=2`, `p=1` minimum, OWASP recommended profile) executed inside `tokio::task::spawn_blocking` to prevent starving async workers.
- Policy (configurable in Settings, strict defaults): minimum 12 characters, mixed character classes, rejected against common weak passwords.
- Zero default credentials: no hardcoded `admin/admin` account ever exists.

### Sessions & API Tokens
- **Browser Sessions**:
  - 32-byte CSPRNG opaque token stored in `meetoops_session` cookie (`HttpOnly`, `SameSite=Lax`, `Path=/`, `Secure` when TLS/reverse-proxy HTTPS detected).
  - Database stores only `SHA-256(token)` so a read-only DB leak cannot be replayed directly into active sessions.
  - Configurable idle and absolute timeout (default: 24h absolute, 8h idle).
  - Password reset or role downgrade immediately revokes all active sessions for that user.
- **API Tokens**:
  - Format: `mto_<40-char-base62>`. Plaintext shown **exactly once** on creation; only `token_prefix` and `SHA-256(token)` are persisted.
  - Scoped to explicit permissions and never allowed to exceed the owning user's role.

### CSRF Protection
- Every authenticated browser session receives a random `csrf_token` returned by `GET /api/v1/auth/me`.
- All cookie-authenticated mutating requests (`POST`, `PUT`, `PATCH`, `DELETE`) MUST include a matching `X-CSRF-Token` header verified in constant time.
- WebSocket upgrades validate the `Origin` header against the configured application URL / Host.

---

## 3. Role-Based Access Control (RBAC)

All authorization checks are enforced **server-side** via Axum extractors/middleware. Client-side UI hiding is strictly for UX convenience.

| Permission | `super_admin` | `admin` | `operator` | `viewer` |
|---|:---:|:---:|:---:|:---:|
| `dashboard:read`, `monitoring:read`, `docker:read`, `logs:read`, `discovery:read` | Yes | Yes | Yes | Yes |
| `docker:operate` (start/stop/restart/pause) | Yes | Yes | Yes | No |
| `docker:delete` (remove container/image/volume/network) | Yes | Yes | No | No |
| `docker:reveal_secrets` (unmask container env vars) | Yes | Yes | No | No |
| `terminal:exec` (interactive container shell) | Yes | Yes | Configurable* | No |
| `db:read`, `redis:read` | Yes | Yes | Yes | Optional* |
| `db:write`, `db:query`, `redis:write`, `redis:console` | Yes | Yes | Configurable* | No |
| `redis:admin` (destructive commands e.g. `FLUSHDB`) | Yes | Yes | No | No |
| `services:credentials` (attach/edit DB & Redis creds) | Yes | Yes | No | No |
| `servers:write`, `servers:delete`, `alerts:write` | Yes | Yes | No | No |
| `users:read`, `users:write`, `users:delete`, `audit:read` | Yes | Yes* | No | No |
| `settings:write`, `backup:manage`, `super_admin` promotion | Yes | No | No | No |

*\*`admin` cannot modify or delete `super_admin` accounts. Per-role optional permissions can be tuned by `super_admin` in Security Settings.*

---

## 4. Secret Encryption & Master Key Management

- **Cipher**: **AES-256-GCM** (authenticated encryption with associated data / AEAD).
- **Associated Data (AAD)**: Bound to `scope:reference_id` so ciphertext cannot be transplanted from one service/record to another.
- **Nonce**: Fresh 96-bit CSPRNG nonce per encryption operation.
- **Master Key Lifecycle**:
  1. On first boot, if `/data/keys/master.key` does not exist, `meetoops-server` generates a 256-bit CSPRNG key, writes it atomically with `0600` permissions, and stores its `SHA-256` fingerprint in `app_state.key_fingerprint`.
  2. On every subsequent boot, the server verifies the loaded master key's fingerprint matches `app_state.key_fingerprint`.
  3. **Fail-Safe Behavior**: If the master key is missing or mismatched on an already-initialized database, the server **refuses to silently overwrite or regenerate the key**. Instead, it enters a safe degraded/locked error state with clear operator recovery instructions (`docs/BACKUP-RESTORE.md`).
  4. In-memory key material uses `zeroize` to clear buffers on drop.

---

## 5. Secret Masking Rules

Container environment variables and connection strings are automatically masked whenever a key name matches (case-insensitive):
- `PASSWORD`, `PASSWD`, `PWD`, `SECRET`, `TOKEN`, `API_KEY`, `APIKEY`, `ACCESS_KEY`, `PRIVATE_KEY`, `CERT`, `CREDENTIAL`, `AUTH`, `DSN`, `CONNECTION_STRING`, `DATABASE_URL`, `REDIS_URL`, `MONGO_URI`

Rules:
- Masked values are replaced server-side with `••••••••` before serialization in `GET /containers/:cid`.
- Explicit reveal requires `docker:reveal_secrets` via `POST /containers/:cid/env/reveal`, returns only the requested container's env, and records an `audit_logs` entry.
- Backend `tracing` logs never print raw request bodies for `/auth/*`, `/setup/admin`, or `/services/*/credentials`.

---

## 6. Remote Agent Security (`meetoops-agent`)

- **Outbound-Only TLS**: Agent initiates `wss://` to MeetoOps Server, verifying the server TLS certificate (with optional custom CA bundle for self-signed internal CAs).
- **Registration Flow**:
  1. Server creates a 256-bit registration token valid for **15 minutes** and **1 use**, bound to a specific pending `server_id`.
  2. Agent presents the registration token once; server invalidates it atomically and issues a unique `agent_id` + 256-bit `agent_secret`.
  3. Subsequent reconnects authenticate using `agent_id` + HMAC-SHA256 challenge-response over TLS (preventing replay even if headers were logged).
- **Revocation**: Deleting a server or clicking **Revoke Agent** in Settings immediately terminates the active agent WebSocket and invalidates `agent_credentials`.

---

## 7. First-Run Setup Security & Recovery

- `POST /api/v1/setup/admin` executes inside an immediate SQLite transaction (`BEGIN IMMEDIATE`) verifying `setup_stage == 'FIRST_RUN_SETUP'` and `COUNT(users) == 0`.
- Once `setup_stage` transitions to `READY`, all `/api/v1/setup/*` write routes permanently return `403 Forbidden`.
- **Admin Lockout Recovery**: First-run setup is NEVER reopened automatically. If all administrators lose access, an operator with host shell access runs the explicit CLI recovery command inside the server container (`meetoops-server reset-admin-password --email <email>`), which requires direct container exec access, hashes a new password, revokes sessions, and writes a system audit entry.

---

## 8. Immutable Audit Logging

Every security-relevant action writes an append-only row to `audit_logs`:
- Setup initialization & completion
- Login success, login failure, account lockout, logout, API token creation/revocation
- User creation, role changes, password resets, deletion
- Container lifecycle mutations, image pulls/removals, volume/network mutations, Compose deploys
- Container secret env reveals & terminal session opens/closes
- Database/Redis credential changes, write queries, destructive Redis commands
- Server/agent registration, revocation, settings changes, and backup/restore operations
