-- Phase 6: Built-in Host SSH Terminal (Termius replacement) & GitHub Projects Deploy

CREATE TABLE IF NOT EXISTS ssh_hosts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_type TEXT NOT NULL DEFAULT 'password',
    use_sudo INTEGER NOT NULL DEFAULT 1,
    provisioned_user TEXT,
    last_banner TEXT,
    last_connected_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    ssh_host_id TEXT NOT NULL REFERENCES ssh_hosts(id) ON DELETE CASCADE,
    repo_url TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'main',
    deploy_path TEXT NOT NULL,
    deploy_command TEXT NOT NULL DEFAULT 'docker compose up -d --build',
    compose_override TEXT,
    has_git_token INTEGER NOT NULL DEFAULT 0,
    has_env_vars INTEGER NOT NULL DEFAULT 0,
    last_deployed_commit TEXT,
    remote_head_commit TEXT,
    last_status TEXT NOT NULL DEFAULT 'idle',
    last_deployed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS project_deployments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    status TEXT NOT NULL,
    commit_hash TEXT,
    triggered_by TEXT,
    log_output TEXT NOT NULL DEFAULT '',
    started_at TEXT NOT NULL,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_project_deployments_project_started
    ON project_deployments (project_id, started_at DESC);
