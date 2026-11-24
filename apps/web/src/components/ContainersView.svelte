<script lang="ts">
  import {
    api,
    type ContainerInspectDto,
    type ContainerStatsSnapshotDto,
    type ContainerSummaryDto,
    type EnvVarDto,
    type PaginationMeta,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import Skeleton from './ui/Skeleton.svelte';
  import {
    RefreshCw,
    Play,
    Square,
    RotateCw,
    Pause,
    Trash2,
    Eye,
    EyeOff,
    ScrollText,
    Terminal,
    Search,
    AlertCircle,
    X,
    Box,
  } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
    canDelete: boolean;
    canRevealSecrets: boolean;
    onOpenLogs: (containerId: string) => void;
    onOpenTerminal?: (containerId: string) => void;
  }

  let { lang, canOperate, canDelete, canRevealSecrets, onOpenLogs, onOpenTerminal }: Props = $props();

  let containers = $state<ContainerSummaryDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let actionBusyId = $state<string | null>(null);
  let searchQuery = $state('');
  let debouncedSearch = $state('');
  let stateFilters = $state<string[]>([]);
  let sortBy = $state('name');
  let sortDir = $state<'asc' | 'desc'>('asc');
  let page = $state(1);
  let perPage = $state(10);

  // Inspector drawer state
  let inspected = $state<ContainerInspectDto | null>(null);
  let inspectTab = $state<'overview' | 'stats' | 'env' | 'mounts' | 'labels'>('overview');
  let containerStats = $state<ContainerStatsSnapshotDto | null>(null);
  let statsLoading = $state(false);
  let revealedEnv = $state<EnvVarDto[] | null>(null);
  let revealing = $state(false);

  $effect(() => {
    const q = searchQuery;
    const timer = setTimeout(() => {
      debouncedSearch = q;
    }, 300);
    return () => clearTimeout(timer);
  });

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      const res = await api.queryContainers({
        all: true,
        search: debouncedSearch || undefined,
        state: stateFilters.join(',') || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      containers = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load containers.';
    } finally {
      loading = false;
    }
  }

  function handleSort(col: string) {
    if (sortBy === col) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    else {
      sortBy = col;
      sortDir = 'asc';
    }
    page = 1;
  }

  $effect(() => {
    void debouncedSearch;
    void stateFilters;
    void sortBy;
    void sortDir;
    void page;
    void perPage;
    load();
  });

  async function runAction(
    c: ContainerSummaryDto,
    action: 'start' | 'stop' | 'restart' | 'pause' | 'unpause',
  ) {
    actionBusyId = c.id;
    errorMsg = null;
    try {
      await api.containerAction(c.id, action);
      toast.success(`Container ${action}ed`, c.name);
      await load();
      if (inspected?.id === c.id) {
        inspected = await api.inspectContainer(c.id);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : `Failed to ${action} container.`;
      errorMsg = msg;
      toast.error(`Failed to ${action} container`, msg);
    } finally {
      actionBusyId = null;
    }
  }

  async function handleRemove(c: ContainerSummaryDto) {
    const ok = await confirmDialog({
      title: 'Remove Container',
      message: 'This action permanently removes the container and cannot be undone.',
      targetName: `${c.name} (${c.short_id})`,
      confirmText: 'Remove Container',
      variant: 'danger',
    });
    if (!ok) return;
    actionBusyId = c.id;
    errorMsg = null;
    try {
      await api.removeContainer(c.id, c.state === 'running');
      toast.success('Container removed', c.name);
      if (inspected?.id === c.id) inspected = null;
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to remove container.';
      errorMsg = msg;
      toast.error('Remove failed', msg);
    } finally {
      actionBusyId = null;
    }
  }

  async function loadContainerStats(id: string) {
    statsLoading = true;
    try {
      containerStats = await api.containerStats(id);
    } catch {
      containerStats = null;
    } finally {
      statsLoading = false;
    }
  }

  async function openInspect(c: ContainerSummaryDto) {
    errorMsg = null;
    revealedEnv = null;
    containerStats = null;
    inspectTab = 'overview';
    try {
      inspected = await api.inspectContainer(c.id);
      if (c.state === 'running') {
        loadContainerStats(c.id);
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to inspect container.';
    }
  }

  function formatBytes(b: number | null): string {
    if (b === null) return '—';
    const mb = b / 1024 ** 2;
    return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`;
  }

  async function handleRevealEnv() {
    if (!inspected) return;
    revealing = true;
    try {
      revealedEnv = await api.revealContainerEnv(inspected.id);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to reveal environment secrets.';
    } finally {
      revealing = false;
    }
  }

  function stateBadgeClass(state: string): string {
    if (state === 'running') return 'bg-[var(--success-subtle)] text-[var(--success)]';
    if (state === 'paused' || state === 'restarting')
      return 'bg-[var(--warning-subtle)] text-[var(--warning)]';
    return 'bg-[var(--danger-subtle)] text-[var(--danger)]';
  }
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">
        {lang === 'id' ? 'Manajemen Kontainer' : 'Containers'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Kelola siklus hidup kontainer, inspeksi detail, masking secret env, dan streaming log.'
          : 'Manage container lifecycle, inspect metadata with secret masking, and jump to live logs.'}
      </p>
    </div>

    <div class="flex items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder={lang === 'id' ? 'Cari nama, image, ID…' : 'Search name, image, ID…'}
          bind:value={searchQuery}
          oninput={() => (page = 1)}
          class="pl-8 pr-3 py-1.5 w-56 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </div>

      <MultiSelect
        class="w-40"
        searchable={false}
        placeholder={lang === 'id' ? 'Status' : 'States'}
        bind:values={stateFilters}
        onchange={() => (page = 1)}
        options={[
          { value: 'running', label: 'Running' },
          { value: 'exited', label: 'Exited' },
          { value: 'paused', label: 'Paused' },
        ]}
      />

      <button
        type="button"
        onclick={load}
        disabled={loading}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-xs text-[var(--foreground)] inline-flex items-center gap-1.5"
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        {lang === 'id' ? 'Muat Ulang' : 'Refresh'}
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-start gap-2.5 p-3.5 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
      <div class="flex-1">{errorMsg}</div>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
    {#if loading && containers.length === 0}
      <TableSkeleton rows={6} columns={6} />
    {:else if containers.length === 0}
      <div class="p-10 text-center space-y-2">
        <Box class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">
          {lang === 'id' ? 'Tidak ada kontainer ditemukan.' : 'No containers found.'}
        </div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
            <th class="px-4 py-2.5">
              <SortHeader label="Name / ID" column="name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Image" column="image" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="State" column="state" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Ports</th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Networks</th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)] text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
          {#each containers as c}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5">
                <button
                  type="button"
                  onclick={() => openInspect(c)}
                  class="text-left hover:text-[var(--primary)] font-medium text-[var(--foreground)] block"
                >
                  {c.name}
                </button>
                <span class="font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">{c.short_id}</span>
              </td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] max-w-[260px] truncate">
                {c.image}
              </td>
              <td class="px-4 py-2.5">
                <span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium {stateBadgeClass(c.state)}">
                  {c.state}
                </span>
                <div class="text-[11px] text-[var(--muted-foreground)] mt-0.5">{c.status}</div>
              </td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                {#if c.ports.length === 0}
                  —
                {:else}
                  {#each c.ports.slice(0, 3) as p}
                    <div>{p.public_port ? `${p.public_port}->` : ''}{p.private_port}/{p.protocol}</div>
                  {/each}
                {/if}
              </td>
              <td class="px-4 py-2.5 text-[11px] text-[var(--muted-foreground)]">
                {c.networks.join(', ') || '—'}
              </td>
              <td class="px-4 py-2.5 text-right whitespace-nowrap">
                <div class="inline-flex items-center gap-1">
                  <button
                    type="button"
                    onclick={() => onOpenLogs(c.id)}
                    title="Logs"
                    class="p-1.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--info)]"
                  >
                    <ScrollText class="w-3.5 h-3.5" />
                  </button>
                  {#if c.state === 'running' && onOpenTerminal}
                    <button
                      type="button"
                      onclick={() => onOpenTerminal(c.id)}
                      title="Terminal"
                      class="p-1.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--primary)]"
                    >
                      <Terminal class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                  {#if canOperate}
                    {#if c.state !== 'running'}
                      <button
                        type="button"
                        disabled={actionBusyId === c.id}
                        onclick={() => runAction(c, 'start')}
                        title="Start"
                        class="p-1.5 rounded hover:bg-[var(--success-subtle)] text-[var(--success)] disabled:opacity-40"
                      >
                        <Play class="w-3.5 h-3.5" />
                      </button>
                    {:else}
                      <button
                        type="button"
                        disabled={actionBusyId === c.id}
                        onclick={() => runAction(c, 'stop')}
                        title="Stop"
                        class="p-1.5 rounded hover:bg-[var(--danger-subtle)] text-[var(--danger)] disabled:opacity-40"
                      >
                        <Square class="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        disabled={actionBusyId === c.id}
                        onclick={() => runAction(c, 'restart')}
                        title="Restart"
                        class="p-1.5 rounded hover:bg-[var(--primary-subtle)] text-[var(--primary)] disabled:opacity-40"
                      >
                        <RotateCw class="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        disabled={actionBusyId === c.id}
                        onclick={() => runAction(c, 'pause')}
                        title="Pause"
                        class="p-1.5 rounded hover:bg-[var(--warning-subtle)] text-[var(--warning)] disabled:opacity-40"
                      >
                        <Pause class="w-3.5 h-3.5" />
                      </button>
                    {/if}
                    {#if c.state === 'paused'}
                      <button
                        type="button"
                        disabled={actionBusyId === c.id}
                        onclick={() => runAction(c, 'unpause')}
                        title="Unpause"
                        class="p-1.5 rounded hover:bg-[var(--success-subtle)] text-[var(--success)] disabled:opacity-40"
                      >
                        <Play class="w-3.5 h-3.5" />
                      </button>
                    {/if}
                  {/if}
                  {#if canDelete}
                    <button
                      type="button"
                      disabled={actionBusyId === c.id}
                      onclick={() => handleRemove(c)}
                      title="Remove"
                      class="p-1.5 rounded hover:bg-[var(--danger-subtle)] text-[var(--danger)] disabled:opacity-40"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <PaginationBar
      page={meta.page}
      perPage={meta.per_page}
      total={meta.total}
      totalPages={meta.total_pages}
      onPageChange={(p) => (page = p)}
      onPerPageChange={(pp) => {
        perPage = pp;
        page = 1;
      }}
    />
  </div>

  <!-- Container Inspector Panel -->
  {#if inspected}
    {@const envList = revealedEnv ?? inspected.env}
    <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] p-5 space-y-4">
      <div class="flex items-center justify-between border-b border-[var(--border)] pb-3">
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-sm font-semibold text-[var(--foreground)]">{inspected.name}</h2>
            <span class="font-mono text-[11px] text-[var(--muted-foreground)]">{inspected.short_id}</span>
            <span class="px-2 py-0.5 rounded-full text-[11px] font-medium {stateBadgeClass(inspected.state)}">
              {inspected.state}
            </span>
          </div>
          <div class="text-[11px] font-mono text-[var(--muted-foreground)] mt-0.5">{inspected.image}</div>
        </div>
        <button
          type="button"
          onclick={() => (inspected = null)}
          class="p-1.5 rounded hover:bg-[var(--surface-elevated)] text-[var(--muted-foreground)]"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="flex gap-2 border-b border-[var(--border)]">
        {#each ['overview', 'stats', 'env', 'mounts', 'labels'] as tName}
          <button
            type="button"
            onclick={() => {
              inspectTab = tName as typeof inspectTab;
              if (tName === 'stats' && inspected) loadContainerStats(inspected.id);
            }}
            class="px-3 py-1.5 text-xs font-medium capitalize border-b-2 {inspectTab === tName ? 'border-[var(--primary)] text-[var(--primary)]' : 'border-transparent text-[var(--muted-foreground)]'}"
          >
            {tName}
          </button>
        {/each}
      </div>

      {#if inspectTab === 'overview'}
        <div class="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
          <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Health Status</div>
            <div class="font-mono text-[var(--foreground)] mt-1">{inspected.health_status ?? 'none'}</div>
          </div>
          <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Restart Policy / Count</div>
            <div class="font-mono text-[var(--foreground)] mt-1 tabular-nums">
              {inspected.restart_policy ?? 'no'} · {inspected.restart_count ?? 0}
            </div>
          </div>
          <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Networks</div>
            <div class="font-mono text-[var(--foreground)] mt-1">{inspected.networks.join(', ') || '—'}</div>
          </div>
        </div>
      {:else if inspectTab === 'stats'}
        {#if statsLoading}
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            {#each [1, 2, 3, 4] as _}
              <Skeleton class="h-16 w-full rounded" />
            {/each}
          </div>
        {:else if containerStats}
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
            <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
              <div class="text-[var(--muted-foreground)]">CPU Usage</div>
              <div class="text-base font-semibold font-mono text-[var(--foreground)] mt-1 tabular-nums">
                {containerStats.cpu_percent !== null ? `${containerStats.cpu_percent.toFixed(1)}%` : '—'}
              </div>
            </div>
            <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
              <div class="text-[var(--muted-foreground)]">Memory</div>
              <div class="text-base font-semibold font-mono text-[var(--foreground)] mt-1 tabular-nums">
                {containerStats.memory_percent !== null ? `${containerStats.memory_percent.toFixed(1)}%` : '—'}
              </div>
              <div class="text-[11px] font-mono text-[var(--muted-foreground)]">
                {formatBytes(containerStats.memory_used_bytes)} / {formatBytes(containerStats.memory_limit_bytes)}
              </div>
            </div>
            <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
              <div class="text-[var(--muted-foreground)]">Network I/O</div>
              <div class="font-mono text-[var(--foreground)] mt-1 tabular-nums">
                ↓ {formatBytes(containerStats.net_rx_bytes)} · ↑ {formatBytes(containerStats.net_tx_bytes)}
              </div>
            </div>
            <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)]">
              <div class="text-[var(--muted-foreground)]">PIDs</div>
              <div class="text-base font-semibold font-mono text-[var(--foreground)] mt-1 tabular-nums">
                {containerStats.pids ?? '—'}
              </div>
            </div>
          </div>
        {:else}
          <div class="p-4 text-xs text-[var(--muted-foreground)] bg-[var(--background)] border border-[var(--border)] rounded">
            Live stats are only available for running containers.
          </div>
        {/if}
      {:else if inspectTab === 'env'}
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs text-[var(--muted-foreground)]">
              Sensitive variables are automatically masked server-side.
            </span>
            {#if canRevealSecrets}
              {#if revealedEnv}
                <button
                  type="button"
                  onclick={() => (revealedEnv = null)}
                  class="px-2.5 py-1 rounded bg-[var(--surface-elevated)] border border-[var(--border)] text-xs inline-flex items-center gap-1.5"
                >
                  <EyeOff class="w-3.5 h-3.5" /> Hide Secrets
                </button>
              {:else}
                <button
                  type="button"
                  disabled={revealing}
                  onclick={handleRevealEnv}
                  class="px-2.5 py-1 rounded bg-[var(--warning-subtle)] border border-[var(--warning)]/40 text-[var(--warning)] text-xs inline-flex items-center gap-1.5"
                >
                  <Eye class="w-3.5 h-3.5" /> Reveal Secrets (Audited)
                </button>
              {/if}
            {/if}
          </div>
          <div class="divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded overflow-hidden">
            {#each envList as ev}
              <div class="px-3 py-2 flex items-center justify-between gap-4 text-xs font-mono bg-[var(--background)]">
                <span class="text-[var(--foreground)] font-medium">{ev.key}</span>
                <span class="{ev.masked ? 'text-[var(--warning)]' : 'text-[var(--muted-foreground)]'} break-all">
                  {ev.value}
                </span>
              </div>
            {/each}
          </div>
        </div>
      {:else if inspectTab === 'mounts'}
        <div class="divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded">
          {#each inspected.mounts as m}
            <div class="px-3 py-2 text-xs font-mono flex flex-wrap justify-between gap-2 bg-[var(--background)]">
              <span class="text-[var(--primary)]">{m.mount_type}</span>
              <span class="text-[var(--foreground)]">{m.source} -> {m.destination}</span>
              <span class="text-[var(--muted-foreground)]">{m.rw ? 'rw' : 'ro'}</span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded">
          {#each Object.entries(inspected.labels) as [k, v]}
            <div class="px-3 py-2 text-xs font-mono flex justify-between gap-4 bg-[var(--background)]">
              <span class="text-[var(--foreground)]">{k}</span>
              <span class="text-[var(--muted-foreground)] break-all">{v}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
