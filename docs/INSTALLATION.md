# MeetoOps — Installation Guide

MeetoOps is engineered for **zero-configuration self-hosted installation**. You do not need to create `.env` files, generate encryption secrets, or run database migrations manually.

---

## 1. System Requirements

| Requirement | Minimum | Recommended (Multi-Server) |
|---|---|---|
| **CPU** | 1 vCPU (x86_64 or arm64) | 2+ vCPU |
| **RAM** | 512 MB | 1–2 GB |
| **Disk** | 2 GB free (SQLite + metrics + logs) | 10 GB+ SSD |
| **Docker** | Docker Engine 24.0+ | Latest stable |
| **Docker Compose** | Compose Plugin v2.20+ | Latest stable |
| **Browser** | Modern Chromium, Firefox, or Safari | Desktop / Laptop |

---

## 2. One-Command Production Installation

```bash
mkdir -p meetoops && cd meetoops

curl -fsSL https://get.meetoops.com/docker-compose.yml \
  -o docker-compose.yml

docker compose up -d
```

### What Happens Automatically on First Boot
1. Docker creates persistent named volumes (`meetoops_data`, `meetoops_keys`).
2. `meetoops-server` initializes the SQLite database in WAL mode and runs all versioned migrations automatically.
3. A 256-bit AEAD master encryption key is generated and persisted safely to `meetoops_keys` (`0600`).
4. The built-in health check (`/api/v1/health`) verifies database and service readiness.
5. Opening `http://<your-server-ip>:8080` launches the **First-Run Setup Wizard**.

---

## 3. Verifying Installation

```bash
docker compose ps
docker compose logs -f meetoops
```

Wait until the container status shows `(healthy)`, then open:

```text
http://<your-server-ip>:8080
```

Proceed to [docs/FIRST-RUN-SETUP.md](./FIRST-RUN-SETUP.md).

---

## 4. Installing Remote Agents (`meetoops-agent`)

To manage additional Docker servers without exposing Docker sockets over the network:
1. Open **Servers -> Add Server -> Remote Agent** in MeetoOps Web (or Step 4 of the Setup Wizard).
2. Copy the generated one-line `docker run` / Compose command containing the time-limited registration token.
3. Run it on the remote host. The agent connects outbound over TLS to MeetoOps Server and appears online automatically.

---

## 5. Lifecycle Commands

- **Stop**: `docker compose down` (preserves all volumes and data)
- **Restart**: `docker compose restart`
- **Upgrade**: `docker compose pull && docker compose up -d` (see [docs/UPGRADE.md](./UPGRADE.md))

> **Warning**: Never run `docker compose down -v` during normal maintenance or upgrades — `-v` permanently deletes the database and encryption key volumes.
