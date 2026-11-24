export type SetupStage =
  | 'UNINITIALIZED'
  | 'FIRST_RUN_SETUP'
  | 'ADMIN_CREATED'
  | 'INFRASTRUCTURE_SETUP'
  | 'READY';

export type Role = 'super_admin' | 'admin' | 'operator' | 'viewer';

export interface SetupStatusDto {
  setup_stage: SetupStage;
  initialized: boolean;
  active_port: number;
  language: string;
  app_name: string;
}

export interface UserPublic {
  id: string;
  full_name: string;
  email: string;
  username: string | null;
  role: Role;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

export interface AuthSessionDto {
  user: UserPublic;
  permissions: string[];
  csrf_token: string;
  setup_stage: SetupStage;
}

export interface LocalDockerProbe {
  available: boolean;
  endpoint: string;
  docker_version: string | null;
  api_version: string | null;
  operating_system: string | null;
  architecture: string | null;
  containers_total: number | null;
  containers_running: number | null;
  containers_stopped: number | null;
  images_total: number | null;
  error_message: string | null;
}

export interface HostMetricsSnapshot {
  hostname: string | null;
  os_name: string | null;
  kernel_version: string | null;
  cpu_cores: number;
  cpu_usage_percent: number | null;
  memory_total_bytes: number | null;
  memory_used_bytes: number | null;
  memory_usage_percent: number | null;
  disk_total_bytes: number | null;
  disk_used_bytes: number | null;
  disk_usage_percent: number | null;
  disk_active_percent?: number | null;
  uptime_seconds: number | null;
}

export interface AuditLogEntry {
  id: string;
  actor_user_id: string | null;
  actor_email: string | null;
  action: string;
  resource_type: string;
  resource_id: string | null;
  server_id: string | null;
  status: string;
  ip_address: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

export interface DashboardSummaryDto {
  servers_total: number;
  servers_online: number;
  servers_offline: number;
  containers_total: number | null;
  containers_running: number | null;
  containers_stopped: number | null;
  containers_unhealthy: number | null;
  host_metrics: HostMetricsSnapshot;
  local_docker: LocalDockerProbe;
  recent_audit_events: AuditLogEntry[];
  active_port: number;
  configured_port: number;
}

export interface AppSettingsDto {
  app_name: string;
  app_url: string;
  language: string;
  timezone: string;
  date_format: string;
  time_format: string;
  monitoring_interval_secs: number;
  session_timeout_hours: number;
  max_login_attempts: number;
  listen_port: number;
  audit_retention_days: number;
}

export interface SettingsResponseDto {
  settings: AppSettingsDto;
  active_port: number;
  restart_required_for_port: boolean;
  vault_fingerprint: string;
  env_port_override: number | null;
}

export interface PortCheckResult {
  port: number;
  available: boolean;
  is_current_port: boolean;
  reason: string;
  suggested_ports: number[];
}

export interface ServerSummary {
  id: string;
  name: string;
  group_name: string;
  connection_type: string;
  endpoint: string | null;
  status: string;
  docker_version: string | null;
  os_info: string | null;
  cpu_cores: number | null;
  memory_total_bytes: number | null;
  last_seen_at: string | null;
  created_at: string;
}

export interface RemoteTokenResponse {
  server: ServerSummary;
  registration_token: string;
  expires_at: string;
  install_command: string;
}

export interface ContainerPortDto {
  ip: string | null;
  private_port: number;
  public_port: number | null;
  protocol: string;
}

export interface ContainerSummaryDto {
  id: string;
  short_id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  created: number | null;
  ports: ContainerPortDto[];
  networks: string[];
  labels: Record<string, string>;
}

export interface EnvVarDto {
  key: string;
  value: string;
  is_secret: boolean;
  masked: boolean;
}

export interface MountDto {
  mount_type: string;
  source: string;
  destination: string;
  mode: string;
  rw: boolean;
}

export interface ContainerInspectDto {
  id: string;
  short_id: string;
  name: string;
  image: string;
  state: string;
  running: boolean;
  paused: boolean;
  restarting: boolean;
  restart_count: number | null;
  health_status: string | null;
  created: string | null;
  started_at: string | null;
  command: string[];
  env: EnvVarDto[];
  mounts: MountDto[];
  networks: string[];
  labels: Record<string, string>;
  restart_policy: string | null;
}

export interface ImageSummaryDto {
  id: string;
  short_id: string;
  repo_tags: string[];
  size_bytes: number;
  created: number;
  containers: number;
}

export interface VolumeSummaryDto {
  name: string;
  driver: string;
  mountpoint: string;
  created_at: string | null;
  scope: string;
  used_by: string[];
}

export interface NetworkSummaryDto {
  id: string;
  short_id: string;
  name: string;
  driver: string;
  scope: string;
  internal: boolean;
  attachable: boolean;
  subnet: string | null;
  gateway: string | null;
  connected_containers: string[];
}

export interface LogLineDto {
  stream: string;
  line: string;
}

export interface HostMetricPoint {
  server_id: string;
  cpu_percent: number | null;
  mem_used_bytes: number | null;
  mem_total_bytes: number | null;
  mem_percent: number | null;
  disk_used_bytes: number | null;
  disk_total_bytes: number | null;
  disk_percent: number | null;
  disk_active_percent: number | null;
  disk_read_bps: number | null;
  disk_write_bps: number | null;
  net_rx_bps: number | null;
  net_tx_bps: number | null;
  uptime_seconds: number | null;
  recorded_at: string;
}

export interface DiscoveredServiceDto {
  id: string;
  server_id: string;
  container_id: string | null;
  name: string;
  service_type: string;
  confidence: string;
  readiness_stage: string;
  host: string | null;
  port: number | null;
  has_credentials: boolean;
  read_only_mode: boolean;
  metadata: Record<string, unknown>;
  last_verified_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface VerifyCredentialsResponse {
  service: DiscoveredServiceDto;
  auth_verified: boolean;
  server_banner: string | null;
  verify_error: string | null;
}

export interface ColumnInfoDto {
  name: string;
  data_type: string;
  is_nullable: boolean;
  default_value: string | null;
}

export interface QueryResultDto {
  columns: string[];
  rows: unknown[][];
  row_count: number;
  truncated: boolean;
}

export interface RedisOverviewDto {
  version: string | null;
  used_memory_human: string | null;
  used_memory_bytes: number | null;
  total_keys: number;
  connected_clients: number | null;
  ops_per_sec: number | null;
  keyspace_hits: number | null;
  keyspace_misses: number | null;
  uptime_seconds: number | null;
}

export interface RedisKeyItemDto {
  key: string;
  key_type: string;
  ttl: number;
}

export interface RedisScanPageDto {
  next_cursor: number;
  keys: RedisKeyItemDto[];
}

export interface RedisKeyDetailDto {
  key: string;
  key_type: string;
  ttl: number;
  value: unknown;
}

export interface ComposeServiceItemDto {
  container_id: string;
  short_id: string;
  container_name: string;
  service_name: string;
  image: string;
  state: string;
  status: string;
}

export interface ComposeStackDto {
  project_name: string;
  working_dir: string | null;
  config_files: string | null;
  total_services: number;
  running_services: number;
  services: ComposeServiceItemDto[];
}

export interface AlertRuleDto {
  id: string;
  name: string;
  server_id: string;
  metric_type: string;
  operator: string;
  threshold: number;
  severity: string;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AlertEventDto {
  id: string;
  rule_id: string;
  rule_name: string;
  server_id: string;
  status: string;
  severity: string;
  message: string;
  metric_value: number | null;
  triggered_at: string;
  acknowledged_by: string | null;
  acknowledged_at: string | null;
  resolved_at: string | null;
}

export interface ApiTokenDto {
  id: string;
  name: string;
  token_prefix: string;
  scopes: string[];
  last_used_at: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface SnapshotInfoDto {
  filename: string;
  size_bytes: number;
  created_at: string;
}

export interface SettingsBackupBundle {
  version: string;
  exported_at: string;
  settings: AppSettingsDto;
  sha256: string;
}

export class ApiClientError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

let csrfToken = '';

export function setCsrfToken(token: string): void {
  csrfToken = token;
}

export interface PaginationMeta {
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

export interface PaginatedResult<T> {
  items: T[];
  meta: PaginationMeta;
}

async function requestPaginated<T>(path: string): Promise<PaginatedResult<T>> {
  const resp = await fetch(path, { credentials: 'include' });
  const payload = await resp.json().catch(() => null);
  if (!resp.ok) {
    const code = payload?.error?.code ?? 'HTTP_ERROR';
    const message = payload?.error?.message ?? `Request failed (${resp.status})`;
    throw new ApiClientError(resp.status, code, message);
  }
  const items = (payload.data ?? []) as T[];
  const meta: PaginationMeta = payload.meta ?? {
    page: 1,
    per_page: items.length || 25,
    total: items.length,
    total_pages: 1,
  };
  return { items, meta };
}

function buildQuery(params: Record<string, string | number | boolean | undefined>): string {
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && String(v).trim() !== '') {
      qs.set(k, String(v));
    }
  }
  const s = qs.toString();
  return s ? `?${s}` : '';
}

export interface PruneResultDto {
  deleted_count: number;
  space_reclaimed_bytes: number;
}

export interface ContainerStatsSnapshotDto {
  cpu_percent: number | null;
  memory_used_bytes: number | null;
  memory_limit_bytes: number | null;
  memory_percent: number | null;
  net_rx_bytes: number | null;
  net_tx_bytes: number | null;
  pids: number | null;
}

export interface AlertChannelDto {
  id: string;
  name: string;
  channel_type: 'webhook' | 'slack' | 'discord' | 'telegram';
  target_url: string;
  is_enabled: boolean;
  created_at: string;
}

export interface AlertsSummaryDto {
  firing: number;
  acknowledged: number;
}

export interface SshHostDto {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: 'password' | 'private_key';
  use_sudo: boolean;
  provisioned_user: string | null;
  last_banner: string | null;
  last_connected_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectDto {
  id: string;
  name: string;
  ssh_host_id: string;
  ssh_host_name: string;
  repo_url: string;
  branch: string;
  deploy_path: string;
  deploy_command: string;
  compose_override: string | null;
  has_git_token: boolean;
  has_env_vars: boolean;
  last_deployed_commit: string | null;
  remote_head_commit: string | null;
  update_available: boolean;
  last_status: string;
  last_deployed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectDeploymentDto {
  id: string;
  project_id: string;
  action: string;
  status: string;
  commit_hash: string | null;
  triggered_by: string | null;
  log_output: string;
  started_at: string;
  finished_at: string | null;
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }
  const method = (init.method ?? 'GET').toUpperCase();
  if (csrfToken && !['GET', 'HEAD', 'OPTIONS'].includes(method)) {
    headers.set('X-CSRF-Token', csrfToken);
  }

  const resp = await fetch(path, {
    ...init,
    headers,
    credentials: 'include',
  });

  const payload = await resp.json().catch(() => null);
  if (!resp.ok) {
    const code = payload?.error?.code ?? 'HTTP_ERROR';
    const message = payload?.error?.message ?? `Request failed (${resp.status})`;
    throw new ApiClientError(resp.status, code, message);
  }

  return payload.data as T;
}

export const api = {
  getSetupStatus: () => request<SetupStatusDto>('/api/v1/setup/status'),
  setupCreateAdmin: (body: {
    full_name: string;
    email: string;
    username?: string;
    password: string;
  }) =>
    request<AuthSessionDto>('/api/v1/setup/admin', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  setupSavePreferences: (body: {
    app_name: string;
    language: string;
    timezone: string;
    date_format: string;
    time_format: string;
    monitoring_interval_secs: number;
  }) =>
    request<AppSettingsDto>('/api/v1/setup/preferences', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  setupTestLocalDocker: () =>
    request<LocalDockerProbe>('/api/v1/setup/docker/test-local', {
      method: 'POST',
    }),
  setupConnectLocalDocker: (name?: string) =>
    request<ServerSummary>('/api/v1/setup/docker/connect-local', {
      method: 'POST',
      body: JSON.stringify({ name }),
    }),
  setupCreateRemoteToken: (name: string, group_name?: string) =>
    request<RemoteTokenResponse>('/api/v1/setup/docker/remote-token', {
      method: 'POST',
      body: JSON.stringify({ name, group_name }),
    }),
  setupComplete: () =>
    request<{ setup_stage: SetupStage }>('/api/v1/setup/complete', {
      method: 'POST',
    }),
  login: (identifier: string, password: string) =>
    request<AuthSessionDto>('/api/v1/auth/login', {
      method: 'POST',
      body: JSON.stringify({ identifier, password }),
    }),
  logout: () => request<{ ok: boolean }>('/api/v1/auth/logout', { method: 'POST' }),
  me: () => request<AuthSessionDto>('/api/v1/auth/me'),
  dashboardSummary: () => request<DashboardSummaryDto>('/api/v1/dashboard/summary'),
  listServers: () => request<ServerSummary[]>('/api/v1/servers'),
  listUsers: () => request<UserPublic[]>('/api/v1/users'),
  queryUsers: (params: {
    search?: string;
    role?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<UserPublic>(`/api/v1/users${buildQuery(params)}`),
  createUser: (body: {
    full_name: string;
    email: string;
    username?: string;
    password: string;
    role: Role;
  }) =>
    request<UserPublic>('/api/v1/users', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  deleteUser: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/users/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  getSettings: () => request<SettingsResponseDto>('/api/v1/settings'),
  updateSettings: (body: AppSettingsDto) =>
    request<SettingsResponseDto>('/api/v1/settings', {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),
  checkPort: (port: number) =>
    request<PortCheckResult>('/api/v1/settings/network/check-port', {
      method: 'POST',
      body: JSON.stringify({ port }),
    }),
  listAuditLogs: (limit = 50) =>
    request<AuditLogEntry[]>(`/api/v1/audit-logs?limit=${limit}`),
  queryAuditLogs: (params: {
    search?: string;
    status?: string;
    resource_type?: string;
    from?: string;
    to?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<AuditLogEntry>(`/api/v1/audit-logs${buildQuery(params)}`),
  // Phase 1: Docker Management
  listContainers: (all = true) =>
    request<ContainerSummaryDto[]>(`/api/v1/docker/containers?all=${all}`),
  queryContainers: (params: {
    all?: boolean;
    search?: string;
    state?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<ContainerSummaryDto>(`/api/v1/docker/containers${buildQuery(params)}`),
  inspectContainer: (id: string) =>
    request<ContainerInspectDto>(`/api/v1/docker/containers/${encodeURIComponent(id)}`),
  containerStats: (id: string) =>
    request<ContainerStatsSnapshotDto>(`/api/v1/docker/containers/${encodeURIComponent(id)}/stats`),
  revealContainerEnv: (id: string) =>
    request<EnvVarDto[]>(`/api/v1/docker/containers/${encodeURIComponent(id)}/env/reveal`, {
      method: 'POST',
    }),
  containerAction: (id: string, action: 'start' | 'stop' | 'restart' | 'pause' | 'unpause') =>
    request<{ ok: boolean }>(`/api/v1/docker/containers/${encodeURIComponent(id)}/${action}`, {
      method: 'POST',
    }),
  removeContainer: (id: string, force = false) =>
    request<{ removed: boolean }>(
      `/api/v1/docker/containers/${encodeURIComponent(id)}?force=${force}`,
      { method: 'DELETE' },
    ),
  containerLogsSnapshot: (id: string, tail = 200) =>
    request<LogLineDto[]>(
      `/api/v1/docker/containers/${encodeURIComponent(id)}/logs?tail=${tail}&timestamps=true`,
    ),
  listImages: () => request<ImageSummaryDto[]>('/api/v1/docker/images'),
  queryImages: (params: {
    search?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<ImageSummaryDto>(`/api/v1/docker/images${buildQuery(params)}`),
  pullImage: (image: string) =>
    request<{ image: string; statuses: string[] }>('/api/v1/docker/images/pull', {
      method: 'POST',
      body: JSON.stringify({ image }),
    }),
  removeImage: (id: string, force = false) =>
    request<{ removed: boolean }>(
      `/api/v1/docker/images/${encodeURIComponent(id)}?force=${force}`,
      { method: 'DELETE' },
    ),
  pruneImages: () => request<PruneResultDto>('/api/v1/docker/images/prune', { method: 'POST' }),
  listVolumes: () => request<VolumeSummaryDto[]>('/api/v1/docker/volumes'),
  queryVolumes: (params: {
    search?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<VolumeSummaryDto>(`/api/v1/docker/volumes${buildQuery(params)}`),
  createVolume: (name: string, driver = 'local') =>
    request<VolumeSummaryDto>('/api/v1/docker/volumes', {
      method: 'POST',
      body: JSON.stringify({ name, driver }),
    }),
  removeVolume: (name: string, force = false) =>
    request<{ removed: boolean }>(
      `/api/v1/docker/volumes/${encodeURIComponent(name)}?force=${force}`,
      { method: 'DELETE' },
    ),
  pruneVolumes: () => request<PruneResultDto>('/api/v1/docker/volumes/prune', { method: 'POST' }),
  listNetworks: () => request<NetworkSummaryDto[]>('/api/v1/docker/networks'),
  queryNetworks: (params: {
    search?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<NetworkSummaryDto>(`/api/v1/docker/networks${buildQuery(params)}`),
  createNetwork: (name: string, driver = 'bridge') =>
    request<{ id: string; name: string }>('/api/v1/docker/networks', {
      method: 'POST',
      body: JSON.stringify({ name, driver }),
    }),
  removeNetwork: (id: string) =>
    request<{ removed: boolean }>(`/api/v1/docker/networks/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  pruneNetworks: () => request<PruneResultDto>('/api/v1/docker/networks/prune', { method: 'POST' }),
  // Phase 2: Infrastructure Monitoring
  hostMetricsHistory: (range: '15m' | '1h' | '6h' | '24h' | '7d' = '1h', serverId = 'local') =>
    request<HostMetricPoint[]>(
      `/api/v1/monitoring/host/history?range=${range}&server_id=${encodeURIComponent(serverId)}`,
    ),
  queryHostMetricLogs: (params: {
    server_id?: string;
    risk?: string;
    from?: string;
    to?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<HostMetricPoint>(`/api/v1/monitoring/host/logs${buildQuery(params)}`),
  // Phase 3: Service Discovery, Database Explorer & Redis Explorer
  listServices: () => request<DiscoveredServiceDto[]>('/api/v1/services'),
  queryServices: (params: {
    search?: string;
    service_type?: string;
    readiness_stage?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<DiscoveredServiceDto>(`/api/v1/services${buildQuery(params)}`),
  scanServices: () => request<DiscoveredServiceDto[]>('/api/v1/services/scan', { method: 'POST' }),
  createManualService: (body: {
    name: string;
    service_type: string;
    host: string;
    port: number;
    read_only_mode?: boolean;
  }) =>
    request<DiscoveredServiceDto>('/api/v1/services', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  saveServiceCredentials: (
    id: string,
    body: {
      host: string;
      port: number;
      username?: string;
      password?: string;
      database?: string;
      redis_db?: number;
      read_only_mode?: boolean;
    },
  ) =>
    request<VerifyCredentialsResponse>(`/api/v1/services/${encodeURIComponent(id)}/credentials`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),
  deleteService: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/services/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  dbListDatabases: (id: string) =>
    request<string[]>(`/api/v1/services/${encodeURIComponent(id)}/db/databases`),
  dbListTables: (id: string, database: string) =>
    request<string[]>(
      `/api/v1/services/${encodeURIComponent(id)}/db/tables?database=${encodeURIComponent(database)}`,
    ),
  dbTableColumns: (id: string, database: string, table: string) =>
    request<ColumnInfoDto[]>(
      `/api/v1/services/${encodeURIComponent(id)}/db/tables/${encodeURIComponent(table)}/columns?database=${encodeURIComponent(database)}`,
    ),
  dbBrowseRows: (id: string, database: string, table: string, limit = 50, offset = 0) =>
    request<QueryResultDto>(
      `/api/v1/services/${encodeURIComponent(id)}/db/tables/${encodeURIComponent(table)}/rows?database=${encodeURIComponent(database)}&limit=${limit}&offset=${offset}`,
    ),
  dbExecuteQuery: (id: string, database: string, sql: string) =>
    request<QueryResultDto>(`/api/v1/services/${encodeURIComponent(id)}/db/query`, {
      method: 'POST',
      body: JSON.stringify({ database, sql }),
    }),
  redisOverview: (id: string) =>
    request<RedisOverviewDto>(`/api/v1/services/${encodeURIComponent(id)}/redis/overview`),
  redisScanKeys: (id: string, cursor = 0, pattern = '*', count = 50) =>
    request<RedisScanPageDto>(
      `/api/v1/services/${encodeURIComponent(id)}/redis/keys?cursor=${cursor}&pattern=${encodeURIComponent(pattern)}&count=${count}`,
    ),
  redisGetKey: (id: string, key: string) =>
    request<RedisKeyDetailDto>(
      `/api/v1/services/${encodeURIComponent(id)}/redis/key?key=${encodeURIComponent(key)}`,
    ),
  redisSetKey: (id: string, body: { key: string; value: string; ttl_secs?: number }) =>
    request<{ saved: boolean }>(`/api/v1/services/${encodeURIComponent(id)}/redis/key`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),
  redisDeleteKey: (id: string, key: string) =>
    request<{ deleted: number }>(
      `/api/v1/services/${encodeURIComponent(id)}/redis/key?key=${encodeURIComponent(key)}`,
      { method: 'DELETE' },
    ),
  redisRenameKey: (id: string, old_key: string, new_key: string) =>
    request<{ renamed: boolean }>(`/api/v1/services/${encodeURIComponent(id)}/redis/rename`, {
      method: 'POST',
      body: JSON.stringify({ old_key, new_key }),
    }),
  redisMutateKey: (
    id: string,
    body: { key: string; key_type: string; field?: string; value: string; score?: number },
  ) =>
    request<{ updated: boolean }>(`/api/v1/services/${encodeURIComponent(id)}/redis/mutate`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  redisCommand: (id: string, command: string, confirm_destructive = false) =>
    request<{ output: string }>(`/api/v1/services/${encodeURIComponent(id)}/redis/command`, {
      method: 'POST',
      body: JSON.stringify({ command, confirm_destructive }),
    }),
  // Phase 4: Compose Stacks, Multi-Server & Alerts
  listStacks: () => request<ComposeStackDto[]>('/api/v1/docker/stacks'),
  stackAction: (project: string, action: 'start' | 'stop' | 'restart') =>
    request<{ affected: number }>(
      `/api/v1/docker/stacks/${encodeURIComponent(project)}/${action}`,
      { method: 'POST' },
    ),
  queryServers: (params: {
    search?: string;
    status?: string;
    connection_type?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<ServerSummary>(`/api/v1/servers/query${buildQuery(params)}`),
  createRemoteServerToken: (name: string, group_name?: string) =>
    request<RemoteTokenResponse>('/api/v1/servers/remote-token', {
      method: 'POST',
      body: JSON.stringify({ name, group_name }),
    }),
  deleteServer: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/servers/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  listAlertRules: () => request<AlertRuleDto[]>('/api/v1/alerts/rules'),
  createAlertRule: (body: {
    name: string;
    metric_type: string;
    operator: string;
    threshold: number;
    severity: string;
  }) =>
    request<AlertRuleDto>('/api/v1/alerts/rules', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  toggleAlertRule: (id: string, is_enabled: boolean) =>
    request<{ ok: boolean }>(`/api/v1/alerts/rules/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify({ is_enabled }),
    }),
  deleteAlertRule: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/alerts/rules/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  queryAlertEvents: (params: {
    search?: string;
    severity?: string;
    status?: string;
    from?: string;
    to?: string;
    sort_by?: string;
    sort_dir?: 'asc' | 'desc';
    page?: number;
    per_page?: number;
  }) => requestPaginated<AlertEventDto>(`/api/v1/alerts/events${buildQuery(params)}`),
  ackAlertEvent: (id: string) =>
    request<{ acknowledged: boolean }>(
      `/api/v1/alerts/events/${encodeURIComponent(id)}/acknowledge`,
      { method: 'POST' },
    ),
  resolveAlertEvent: (id: string) =>
    request<{ resolved: boolean }>(
      `/api/v1/alerts/events/${encodeURIComponent(id)}/resolve`,
      { method: 'POST' },
    ),
  alertsSummary: () => request<AlertsSummaryDto>('/api/v1/alerts/summary'),
  listAlertChannels: () => request<AlertChannelDto[]>('/api/v1/alerts/channels'),
  createAlertChannel: (body: { name: string; channel_type: string; target_url: string }) =>
    request<AlertChannelDto>('/api/v1/alerts/channels', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  toggleAlertChannel: (id: string, is_enabled: boolean) =>
    request<{ ok: boolean }>(`/api/v1/alerts/channels/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify({ is_enabled }),
    }),
  deleteAlertChannel: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/alerts/channels/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  testAlertChannel: (id: string) =>
    request<{ delivered: boolean }>(`/api/v1/alerts/channels/${encodeURIComponent(id)}/test`, {
      method: 'POST',
    }),
  // Phase 5: API Tokens & Backup/Restore
  listApiTokens: () => request<ApiTokenDto[]>('/api/v1/auth/tokens'),
  createApiToken: (name: string, expires_days?: number) =>
    request<{ token: ApiTokenDto; plaintext_token: string }>('/api/v1/auth/tokens', {
      method: 'POST',
      body: JSON.stringify({ name, expires_days }),
    }),
  revokeApiToken: (id: string) =>
    request<{ revoked: boolean }>(`/api/v1/auth/tokens/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  listDbSnapshots: () => request<SnapshotInfoDto[]>('/api/v1/backup/snapshots'),
  createDbSnapshot: () =>
    request<SnapshotInfoDto>('/api/v1/backup/snapshots', { method: 'POST' }),
  exportSettingsBackup: () => request<SettingsBackupBundle>('/api/v1/backup/export-settings'),
  importSettingsBackup: (bundle: SettingsBackupBundle) =>
    request<{ restored: boolean }>('/api/v1/backup/import-settings', {
      method: 'POST',
      body: JSON.stringify(bundle),
    }),
  // Phase 6: Host SSH Terminal & GitHub Projects Deploy
  listSshHosts: () => request<SshHostDto[]>('/api/v1/ssh/hosts'),
  createSshHost: (body: {
    name: string;
    host: string;
    port?: number;
    username: string;
    auth_type: 'password' | 'private_key';
    password?: string;
    private_key_pem?: string;
    passphrase?: string;
    use_sudo?: boolean;
  }) =>
    request<{ host: SshHostDto; verified: boolean; banner: string | null; verify_error: string | null }>(
      '/api/v1/ssh/hosts',
      {
        method: 'POST',
        body: JSON.stringify(body),
      },
    ),
  testSshHost: (id: string) =>
    request<{ connected: boolean; banner: string }>(
      `/api/v1/ssh/hosts/${encodeURIComponent(id)}/test`,
      { method: 'POST' },
    ),
  provisionSshUser: (
    id: string,
    body: { linux_username?: string; password: string; add_docker_group?: boolean },
  ) =>
    request<{ host: SshHostDto; linux_username: string; id_output: string }>(
      `/api/v1/ssh/hosts/${encodeURIComponent(id)}/provision-user`,
      {
        method: 'POST',
        body: JSON.stringify(body),
      },
    ),
  deleteSshHost: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/ssh/hosts/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  listProjects: () => request<ProjectDto[]>('/api/v1/projects'),
  createProject: (body: {
    name: string;
    ssh_host_id: string;
    repo_url: string;
    branch?: string;
    deploy_path: string;
    deploy_command?: string;
    compose_override?: string;
    git_token?: string;
    env_content?: string;
  }) =>
    request<ProjectDto>('/api/v1/projects', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  updateProject: (
    id: string,
    body: {
      branch?: string;
      deploy_command?: string;
      compose_override?: string;
      git_token?: string;
      env_content?: string;
    },
  ) =>
    request<ProjectDto>(`/api/v1/projects/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),
  revealProjectEnv: (id: string) =>
    request<{ env_content: string }>(`/api/v1/projects/${encodeURIComponent(id)}/env`),
  deleteProject: (id: string) =>
    request<{ deleted: boolean }>(`/api/v1/projects/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }),
  checkProjectRemote: (id: string) =>
    request<ProjectDto>(`/api/v1/projects/${encodeURIComponent(id)}/check-remote`, {
      method: 'POST',
    }),
  deployProject: (id: string) =>
    request<ProjectDeploymentDto>(`/api/v1/projects/${encodeURIComponent(id)}/deploy`, {
      method: 'POST',
    }),
  projectAction: (id: string, action: 'restart' | 'stop' | 'logs') =>
    request<ProjectDeploymentDto>(
      `/api/v1/projects/${encodeURIComponent(id)}/action/${action}`,
      { method: 'POST' },
    ),
  listProjectDeployments: (id: string) =>
    request<ProjectDeploymentDto[]>(`/api/v1/projects/${encodeURIComponent(id)}/deployments`),
};
