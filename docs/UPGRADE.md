# MeetoOps — Upgrade & Migration Guide

MeetoOps is designed for safe, zero-data-loss upgrades with automatic versioned schema migrations.

---

## 1. Standard Upgrade Procedure

```bash
cd meetoops
docker compose pull
docker compose up -d
```

### What Is Preserved Across Upgrades
Because all persistent state lives in named Docker volumes (`meetoops_data` and `meetoops_keys`), upgrading preserves:
- Administrator & user accounts and roles
- Application settings & preferences
- Registered local and remote servers & agent credentials
- Encrypted database and Redis credentials
- Master encryption key
- Historical monitoring metrics, alert rules, and audit logs

---

## 2. Automatic Migration Safety

1. On startup with a newer image, `meetoops-server` checks `_sqlx_migrations` against embedded migrations.
2. Before applying new migrations, `meetoops-server` creates an automatic pre-upgrade SQLite snapshot in `/data/db/backups/pre-migration-<timestamp>.db`.
3. Migrations execute transactionally; if any migration fails, startup aborts safely without corrupting the existing database.
4. Check logs after upgrading:
   ```bash
   docker compose logs --tail=100 meetoops
   ```

---

## 3. Pre-Upgrade Best Practices

- Before major version upgrades (`v1.x` -> `v2.x`), export a backup via **Settings -> Backup & Restore** (or back up both `meetoops_data` and `meetoops_keys`).
- **Never** run `docker compose down -v` during an upgrade.
- Upgrade `meetoops-agent` instances on remote hosts after upgrading `meetoops-server`; the agent protocol maintains backward compatibility across minor versions.
