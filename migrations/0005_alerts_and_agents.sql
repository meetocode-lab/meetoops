-- MeetoOps Phase 4: Alert Rules, Alert Events, and Remote Agent Credentials
-- Forward-only migration 0005

CREATE TABLE IF NOT EXISTS alert_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    server_id TEXT NOT NULL DEFAULT 'local',
    metric_type TEXT NOT NULL,
    operator TEXT NOT NULL DEFAULT 'gte',
    threshold REAL NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alert_events (
    id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    rule_name TEXT NOT NULL,
    server_id TEXT NOT NULL DEFAULT 'local',
    status TEXT NOT NULL DEFAULT 'firing',
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    metric_value REAL NULL,
    triggered_at TEXT NOT NULL,
    acknowledged_by TEXT NULL,
    acknowledged_at TEXT NULL,
    resolved_at TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_alert_events_status ON alert_events(status, triggered_at DESC);
CREATE INDEX IF NOT EXISTS idx_alert_events_rule ON alert_events(rule_id, status);

CREATE TABLE IF NOT EXISTS agent_credentials (
    server_id TEXT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    agent_secret_hash TEXT NOT NULL,
    rotated_at TEXT NOT NULL,
    revoked_at TEXT NULL
);
