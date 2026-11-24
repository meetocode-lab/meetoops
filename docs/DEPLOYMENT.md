# MeetoOps — Production Deployment Guide

## 1. Standard Docker Compose Deployment

MeetoOps ships as a zero-config single-container Compose stack exposing port `9480` by default (chosen to avoid collisions with `8080`, `3000`, and `9000`; configurable in Admin Settings with built-in port availability probing).

```yaml
services:
  meetoops:
    image: ghcr.io/meetoops/meetoops:latest
    container_name: meetoops
    restart: unless-stopped
    ports:
      - "127.0.0.1:9480:9480" # Use "9480:9480" if not behind a local reverse proxy
    volumes:
      - meetoops_data:/data
      - meetoops_keys:/data/keys
      - /var/run/docker.sock:/var/run/docker.sock # Optional: omit if managing remote agents only
    security_opt:
      - no-new-privileges:true
    healthcheck:
      test: ["CMD", "/usr/local/bin/meetoops-server", "healthcheck"]
      interval: 15s
      timeout: 5s
      retries: 3
      start_period: 10s

volumes:
  meetoops_data:
  meetoops_keys:
```

---

## 2. Reverse Proxy & HTTPS (Recommended)

Always terminate TLS in production so session cookies are marked `Secure` and WebSockets (`logs`, `terminal`, `realtime`, `agent`) run over `wss://`. Set `MEETOOPS_TRUST_PROXY=true` when behind a reverse proxy.

### Caddy Example (Automatic Let's Encrypt)
```caddyfile
ops.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

### Nginx Example (with WebSocket Upgrade)
```nginx
server {
    listen 443 ssl http2;
    server_name ops.example.com;

    ssl_certificate     /etc/letsencrypt/live/ops.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ops.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 3600s;
    }
}
```

---

## 3. Graceful Shutdown & Readiness

- `meetoops-server` handles `SIGTERM` / `SIGINT` cleanly: stops accepting new connections, closes active WebSocket streams, flushes SQLite WAL checkpoints, and exits `0`.
- Readiness is exposed at `GET /api/v1/health` (returns `200 OK` only after migrations complete and the encryption vault is verified).
