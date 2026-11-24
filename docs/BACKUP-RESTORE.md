# MeetoOps — Backup & Restore Guide

MeetoOps stores persistent state in two places:
1. **`meetoops_data` (`/data`)**: SQLite database (`meetoops.db`), settings, metrics, audit logs, and encrypted secrets.
2. **`meetoops_keys` (`/data/keys/master.key`)**: 256-bit AEAD master key required to decrypt `encrypted_secrets`.

> **Critical**: Restoring `meetoops_data` without the matching `master.key` will prevent MeetoOps from decrypting stored database/Redis credentials. Always back up both — ideally in separate secure locations.

---

## 1. Web UI Backup (Recommended for Settings & DB)

Navigate to **Settings -> Backup & Restore** (`super_admin` required):
- **Export Database & Configuration Snapshot**: Creates a consistent online SQLite backup (`VACUUM INTO`) bundled with a SHA-256 manifest checksum.
- **Export Settings Only (JSON)**: Exports non-secret application settings for replication across environments.
- **Validate Backup**: Verifies archive structure, schema version compatibility, and checksum integrity before restoring.

---

## 2. Host-Level Volume Backup (Docker)

```bash
# 1. Stop container briefly for a cold snapshot (or use Web UI hot backup)
docker compose stop meetoops

# 2. Archive data volume
docker run --rm -v meetoops_data:/data -v "$(pwd)/backups:/backup" alpine \
  tar czf /backup/meetoops-data-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .

# 3. Archive master key volume (store separately in a secure vault!)
docker run --rm -v meetoops_keys:/keys -v "$(pwd)/backups:/backup" alpine \
  tar czf /backup/meetoops-keys-$(date +%Y%m%d-%H%M%S).tar.gz -C /keys .

# 4. Restart MeetoOps
docker compose start meetoops
```

---

## 3. Restore Procedure

1. Stop MeetoOps: `docker compose down` (do **not** use `-v` unless replacing volumes intentionally).
2. Restore both `meetoops_data` and `meetoops_keys` archives to their respective named volumes.
3. Verify `/data/keys/master.key` has `0600` permissions.
4. Start MeetoOps: `docker compose up -d`.
5. Confirm `GET /api/v1/health` returns `healthy` and verify stored service connections in **Service Discovery**.

---

## 4. Missing or Mismatched Master Key Recovery

If `meetoops-server` detects that `master.key` is missing or does not match `app_state.key_fingerprint`:
- The server refuses to auto-regenerate a new key (preventing silent permanent loss of encrypted credentials).
- **Option A (Preferred)**: Restore the original `master.key` from your secure backup into `/data/keys/master.key` and restart.
- **Option B (Emergency Reset)**: If the original key is permanently lost, an operator with host access may run `meetoops-server vault reset-key --confirm`, which generates a new master key, clears undecryptable rows in `encrypted_secrets`, and records an audit event so credentials can be re-entered cleanly.
