<script lang="ts">
  import { api, type DashboardSummaryDto } from '../lib/api';
  import { translations, type Language } from '../lib/i18n';
  import Skeleton from './ui/Skeleton.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import { RefreshCw, Server, Box, Cpu, MemoryStick, HardDrive, Clock, AlertCircle } from 'lucide-svelte';

  interface Props {
    lang: Language;
  }

  let { lang }: Props = $props();
  const t = $derived(translations[lang]);

  let summary = $state<DashboardSummaryDto | null>(null);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let wsConnected = $state(false);
  let liveDiskActive = $state<number | null>(null);

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      summary = await api.dashboardSummary();
      liveDiskActive = summary.host_metrics.disk_active_percent ?? null;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load dashboard telemetry.';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${proto}//${window.location.host}/ws/v1/monitoring/realtime`);
    ws.onopen = () => {
      wsConnected = true;
    };
    ws.onclose = () => {
      wsConnected = false;
    };
    ws.onmessage = (ev) => {
      try {
        const pt = JSON.parse(ev.data);
        if (summary && pt) {
          summary = {
            ...summary,
            host_metrics: {
              ...summary.host_metrics,
              cpu_usage_percent: pt.cpu_percent ?? summary.host_metrics.cpu_usage_percent,
              memory_used_bytes: pt.mem_used_bytes ?? summary.host_metrics.memory_used_bytes,
              memory_total_bytes: pt.mem_total_bytes ?? summary.host_metrics.memory_total_bytes,
              memory_usage_percent: pt.mem_percent ?? summary.host_metrics.memory_usage_percent,
              disk_used_bytes: pt.disk_used_bytes ?? summary.host_metrics.disk_used_bytes,
              disk_total_bytes: pt.disk_total_bytes ?? summary.host_metrics.disk_total_bytes,
              disk_usage_percent: pt.disk_percent ?? summary.host_metrics.disk_usage_percent,
              disk_active_percent: pt.disk_active_percent ?? summary.host_metrics.disk_active_percent,
              uptime_seconds: pt.uptime_seconds ?? summary.host_metrics.uptime_seconds,
            },
          };
          if (pt.disk_active_percent !== undefined) {
            liveDiskActive = pt.disk_active_percent;
          }
        }
      } catch {
        // ignore malformed ws frame
      }
    };
    return () => {
      ws.close();
    };
  });

  function formatBytes(bytes: number | null): string {
    if (bytes === null) return '—';
    const gb = bytes / (1024 ** 3);
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = bytes / (1024 ** 2);
    return `${mb.toFixed(0)} MB`;
  }

  function formatUptime(secs: number | null): string {
    if (secs === null) return '—';
    const days = Math.floor(secs / 86400);
    const hours = Math.floor((secs % 86400) / 3600);
    const mins = Math.floor((secs % 3600) / 60);
    return days > 0 ? `${days}d ${hours}h` : `${hours}h ${mins}m`;
  }
</script>

<div class="space-y-5">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">{t.dashboard.title}</h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">{t.dashboard.subtitle}</p>
    </div>
    <div class="flex items-center gap-2">
      <span
        class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium border border-[var(--border)] bg-[var(--surface)] text-[var(--muted-foreground)]"
      >
        <span class="w-1.5 h-1.5 rounded-full {wsConnected ? 'bg-[var(--success)] animate-pulse' : 'bg-[var(--muted-foreground)]'}"></span>
        {wsConnected ? 'Live' : 'Connecting…'}
      </span>
      <button
        type="button"
        onclick={load}
        disabled={loading}
        class="px-3 py-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-xs text-[var(--foreground)] inline-flex items-center gap-1.5 disabled:opacity-50"
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        {t.dashboard.refresh}
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center justify-between p-3.5 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <div class="flex items-center gap-2">
        <AlertCircle class="w-4 h-4" />
        <span>{errorMsg}</span>
      </div>
      <button type="button" onclick={load} class="underline font-medium">{t.retry}</button>
    </div>
  {:else if loading && !summary}
    <div class="space-y-4">
      <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-3">
        {#each [1, 2, 3, 4, 5, 6] as _}
          <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
            <Skeleton class="h-3 w-20" />
            <Skeleton class="h-7 w-16" />
            <Skeleton class="h-3 w-28" />
          </div>
        {/each}
      </div>
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
          <Skeleton class="h-4 w-36" />
          <Skeleton class="h-3.5 w-full" />
          <Skeleton class="h-3.5 w-5/6" />
          <Skeleton class="h-3.5 w-2/3" />
        </div>
        <div class="lg:col-span-2 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-hidden">
          <TableSkeleton rows={5} columns={4} />
        </div>
      </div>
    </div>
  {:else if summary}
    <!-- Primary KPI Strip (Real Host & Docker Telemetry — null shown as —, never 0) -->
    <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-3">
      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.serversTotal}</span>
          <Server class="w-3.5 h-3.5 text-[var(--primary)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {summary.servers_total}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums mt-1">
          <span class="text-[var(--success)]">{summary.servers_online} online</span> · {summary.servers_offline} offline
        </div>
      </div>

      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.containersTotal}</span>
          <Box class="w-3.5 h-3.5 text-[var(--info)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {summary.containers_total ?? '—'}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums mt-1">
          {#if summary.containers_total !== null}
            <span class="text-[var(--success)]">{summary.containers_running ?? 0} running</span> · {summary.containers_stopped ?? 0} stopped
          {:else}
            {t.unavailable}
          {/if}
        </div>
      </div>

      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.hostCpu}</span>
          <Cpu class="w-3.5 h-3.5 text-[var(--primary)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {summary.host_metrics.cpu_usage_percent !== null ? `${summary.host_metrics.cpu_usage_percent.toFixed(1)}%` : '—'}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums mt-1">
          {summary.host_metrics.cpu_cores} cores
        </div>
      </div>

      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.hostMemory}</span>
          <MemoryStick class="w-3.5 h-3.5 text-[var(--info)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {summary.host_metrics.memory_usage_percent !== null ? `${summary.host_metrics.memory_usage_percent.toFixed(1)}%` : '—'}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums mt-1">
          {formatBytes(summary.host_metrics.memory_used_bytes)} / {formatBytes(summary.host_metrics.memory_total_bytes)}
        </div>
      </div>

      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.hostDisk}</span>
          <HardDrive class="w-3.5 h-3.5 text-[var(--secondary)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {liveDiskActive !== null
            ? `${liveDiskActive.toFixed(1)}%`
            : summary.host_metrics.disk_usage_percent !== null
              ? `${summary.host_metrics.disk_usage_percent.toFixed(1)}%`
              : '—'}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums mt-1">
          Space {summary.host_metrics.disk_usage_percent !== null ? `${summary.host_metrics.disk_usage_percent.toFixed(1)}%` : '—'} · {formatBytes(summary.host_metrics.disk_used_bytes)} / {formatBytes(summary.host_metrics.disk_total_bytes)}
        </div>
      </div>

      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          <span>{t.dashboard.hostUptime}</span>
          <Clock class="w-3.5 h-3.5 text-[var(--success)]" />
        </div>
        <div class="text-2xl font-semibold text-[var(--foreground)] tabular-nums mt-2">
          {formatUptime(summary.host_metrics.uptime_seconds)}
        </div>
        <div class="text-[11px] text-[var(--muted-foreground)] truncate mt-1">
          {summary.host_metrics.hostname ?? '—'}
        </div>
      </div>
    </div>

    <!-- Docker Engine Status & Recent Audit Events -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
        <div class="flex items-center justify-between">
          <h2 class="text-sm font-semibold text-[var(--foreground)]">{t.dashboard.dockerEngine}</h2>
          <span
            class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium {summary.local_docker.available ? 'bg-[var(--success-subtle)] text-[var(--success)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)]'}"
          >
            <span class="w-1.5 h-1.5 rounded-full {summary.local_docker.available ? 'bg-[var(--success)]' : 'bg-[var(--warning)]'}"></span>
            {summary.local_docker.available ? 'Connected' : t.unavailable}
          </span>
        </div>

        <div class="space-y-1.5 text-xs">
          <div class="flex justify-between py-1 border-b border-[var(--border-subtle)]">
            <span class="text-[var(--muted-foreground)]">Endpoint</span>
            <span class="font-mono text-[var(--foreground)]">{summary.local_docker.endpoint}</span>
          </div>
          <div class="flex justify-between py-1 border-b border-[var(--border-subtle)]">
            <span class="text-[var(--muted-foreground)]">Version</span>
            <span class="font-mono text-[var(--foreground)]">{summary.local_docker.docker_version ?? '—'}</span>
          </div>
          <div class="flex justify-between py-1 border-b border-[var(--border-subtle)]">
            <span class="text-[var(--muted-foreground)]">Host OS</span>
            <span class="text-[var(--foreground)] truncate max-w-[200px]">{summary.host_metrics.os_name ?? '—'}</span>
          </div>
          <div class="flex justify-between py-1">
            <span class="text-[var(--muted-foreground)]">Control Port</span>
            <span class="font-mono text-[var(--primary)] tabular-nums">:{summary.active_port}</span>
          </div>
        </div>

        {#if !summary.local_docker.available && summary.local_docker.error_message}
          <div class="p-2.5 rounded-[var(--radius-sm)] bg-[var(--background)] border border-[var(--border)] text-[11px] font-mono text-[var(--muted-foreground)] break-all">
            {summary.local_docker.error_message}
          </div>
        {/if}
      </div>

      <div class="lg:col-span-2 p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
        <h2 class="text-sm font-semibold text-[var(--foreground)]">{t.dashboard.recentAudit}</h2>
        {#if summary.recent_audit_events.length === 0}
          <p class="text-xs text-[var(--muted-foreground)] py-6 text-center">{t.dashboard.noEvents}</p>
        {:else}
          <div class="overflow-x-auto">
            <table class="w-full text-left border-collapse">
              <thead>
                <tr class="border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
                  <th class="py-2 pr-3 font-medium">Action</th>
                  <th class="py-2 pr-3 font-medium">Actor</th>
                  <th class="py-2 pr-3 font-medium">Status</th>
                  <th class="py-2 font-medium text-right">Time (UTC)</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
                {#each summary.recent_audit_events as ev}
                  <tr class="hover:bg-[var(--surface-elevated)]">
                    <td class="py-2 pr-3 font-mono text-[var(--foreground)]">{ev.action}</td>
                    <td class="py-2 pr-3 text-[var(--muted-foreground)]">{ev.actor_email ?? 'system'}</td>
                    <td class="py-2 pr-3">
                      <span class="px-1.5 py-0.5 rounded text-[11px] {ev.status === 'success' ? 'bg-[var(--success-subtle)] text-[var(--success)]' : 'bg-[var(--danger-subtle)] text-[var(--danger)]'}">
                        {ev.status}
                      </span>
                    </td>
                    <td class="py-2 text-right font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                      {ev.created_at.slice(0, 19).replace('T', ' ')}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
