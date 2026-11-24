-- MeetoOps Phase 3: Smart Service Detection & Explorer Connections
-- Forward-only migration 0003

CREATE TABLE IF NOT EXISTS discovered_services (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL DEFAULT 'local',
    container_id TEXT NULL,
    name TEXT NOT NULL,
    service_type TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'unknown',
    readiness_stage TEXT NOT NULL DEFAULT 'detected',
    host TEXT NULL,
    port INTEGER NULL,
    secret_id TEXT NULL REFERENCES encrypted_secrets(id) ON DELETE SET NULL,
    read_only_mode INTEGER NOT NULL DEFAULT 1,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    last_verified_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discovered_services_server ON discovered_services(server_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_discovered_services_container
    ON discovered_services(server_id, container_id)
    WHERE container_id IS NOT NULL;
