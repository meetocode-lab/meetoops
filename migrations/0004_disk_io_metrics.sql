-- MeetoOps: Add Disk Active Time (%) and Disk Read/Write throughput (bytes/sec)
-- Forward-only migration 0004

ALTER TABLE metrics_host ADD COLUMN disk_active_percent REAL NULL;
ALTER TABLE metrics_host ADD COLUMN disk_read_bps REAL NULL;
ALTER TABLE metrics_host ADD COLUMN disk_write_bps REAL NULL;
