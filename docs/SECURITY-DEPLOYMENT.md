# MeetoOps — Deployment Security Guide

This guide covers host-level and network-level security considerations for deploying MeetoOps in production.

---

## 1. Local Docker Socket Security Model

Mounting `/var/run/docker.sock` into a container grants **effective root-equivalent privileges on the Docker host** (because any Docker API client can mount the host filesystem into a privileged container).

### MeetoOps Mitigations
- **Never expose the Docker socket to any network interface** (never enable unauthenticated TCP `2375`).
- In the default zero-config Compose deployment, `/var/run/docker.sock` is mounted **only** into `meetoops-server` (or an isolated local `meetoops-agent`) and is never directly accessible to the browser or unauthenticated routes.
- **Optional Socket Proxy Hardening**: Security-conscious teams can place a read/write-filtered Docker socket proxy (e.g., `tecnativa/docker-socket-proxy`) on an internal-only Docker network and point `MEETOOPS_DOCKER_HOST=tcp://socket-proxy:2375` without mounting the raw socket into `meetoops-server`.
- **Remote Hosts**: Always use `meetoops-agent` with outbound TLS instead of exposing remote Docker daemons over TCP.

---

## 2. Network Exposure & Reverse Proxy (HTTPS)

- The default `docker-compose.yml` exposes only **one port**: `8080` (MeetoOps Web + API).
- The internal SQLite database and master encryption key live on private named Docker volumes with **zero exposed ports**.
- **Production HTTPS Requirement**: Always place MeetoOps behind a TLS-terminating reverse proxy (Caddy, Nginx, Traefik, or Cloudflare Tunnel) so session cookies use the `Secure` attribute and WebSocket traffic uses `wss://`.
- Restrict port `8080` to `127.0.0.1:8080:8080` on the host when running behind a local reverse proxy, or place MeetoOps behind a VPN/Tailscale/WireGuard network for private infrastructure access.

---

## 3. Volume & Encryption Key Protection

MeetoOps uses two distinct persistent volumes:
1. `meetoops_data` (`/data/db`, `/data/config`) — Application database and runtime state.
2. `meetoops_keys` (`/data/keys`) — Auto-generated 256-bit `master.key` (`0600`).

### Recommendations
- Back up `meetoops_keys` **separately** from `meetoops_data` in a secure secret manager or encrypted offline vault.
- Never commit `master.key` to Git or store it in unencrypted cloud buckets alongside the SQLite database.
- Restrict host filesystem permissions on Docker volume directories to `root` / container UID (`10001`).

---

## 4. Container Runtime Hardening

Production container images for `meetoops-server` and `meetoops-agent` follow hardened defaults:
- Multi-stage minimal runtime image (distroless or slim Debian/Alpine without build toolchains).
- Non-root user (`meetoops`, UID `10001`) by default (with optional Docker GID mapping when local socket access is enabled).
- `no-new-privileges: true` security option enabled in Compose.
- Strict HTTP security headers (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Content-Security-Policy`).

---

## 5. Pre-Production Security Checklist

- [ ] First-Run Setup completed and `/api/v1/setup/status` reports `READY`.
- [ ] HTTPS / TLS enabled via reverse proxy (`wss://` verified for WebSockets).
- [ ] Port `8080` bound to `127.0.0.1` or protected by firewall/VPN.
- [ ] Strong Super Administrator password configured; least-privilege roles (`operator`, `viewer`) assigned to team members.
- [ ] Database Explorer connections left in **Read-Only** mode unless write access is explicitly needed.
- [ ] Master key (`meetoops_keys`) backed up securely offline.
- [ ] Audit log retention and login rate-limit thresholds reviewed in **Settings -> Security**.
