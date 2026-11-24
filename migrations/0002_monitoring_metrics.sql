-- MeetoOps Phase 2: Historical Infrastructure Monitoring Metrics
-- Forward-only migration 0002
-- All telemetry columns are NULLABLE: missing metrics are stored as NULL, never 0.

CREATE TABLE IF NOT EXISTS metrics_host (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id TEXT NOT NULL DEFAULT 'local',
    cpu_percent REAL NULL,
    mem_used_bytes INTEGER NULL,
    mem_total_bytes INTEGER NULL,
    mem_percent REAL NULL,
    disk_used_bytes INTEGER NULL,
    disk_total_bytes INTEGER NULL,
    disk_percent REAL NULL,
    net_rx_bps REAL NULL,
    net_tx_bps REAL NULL,
    uptime_seconds INTEGER NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metrics_host_server_time
    ON metrics_host(server_id, recorded_at DESC);
