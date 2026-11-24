# Stage 1: Build Svelte 5 Web UI with Bun
FROM oven/bun:1-alpine AS web-builder
WORKDIR /src/apps/web
COPY apps/web/package.json apps/web/bun.lock* ./
RUN bun install --frozen-lockfile || bun install
COPY apps/web ./
RUN bun run build

# Stage 2: Build Rust Server binary
FROM rust:1-slim-bookworm AS rust-builder
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY services ./services
COPY migrations ./migrations
RUN cargo build --release -p meetoops-server

# Stage 3: Minimal Production Runtime (Single Container, Zero-Config)
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-builder /src/target/release/meetoops-server /usr/local/bin/meetoops-server
COPY --from=web-builder /src/apps/web/dist /app/web

RUN mkdir -p /data/db /data/keys

ENV MEETOOPS_DATA_DIR=/data
ENV MEETOOPS_WEB_DIR=/app/web
EXPOSE 9480

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/meetoops-server", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/meetoops-server"]
