-- MeetoOps: External Alert Notification Channels (Webhook, Slack, Discord, Telegram)
-- Forward-only migration 0006

CREATE TABLE IF NOT EXISTS alert_channels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL,
    target_url TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);
