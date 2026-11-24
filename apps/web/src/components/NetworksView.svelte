<script lang="ts">
  import { api, type NetworkSummaryDto, type PaginationMeta } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import { RefreshCw, Plus, Trash2, AlertCircle, Network, Search, Sparkles } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
    canDelete: boolean;
  }

  let { lang, canOperate, canDelete }: Props = $props();

  let networks = $state<NetworkSummaryDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let newName = $state('');
  let newDriver = $state('bridge');
  let searchQuery = $state('');
  let debouncedSearch = $state('');
  let sortBy = $state('name');
  let sortDir = $state<'asc' | 'desc'>('asc');
  let page = $state(1);
  let perPage = $state(10);
  let errorMsg = $state<string | null>(null);

  $effect(() => {
    const q = searchQuery;
    const t = setTimeout(() => (debouncedSearch = q), 300);
    return () => clearTimeout(t);
  });

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      const res = await api.queryNetworks({
        search: debouncedSearch || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      networks = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load networks.';
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
    void sortBy;
    void sortDir;
    void page;
    void perPage;
    load();
  });

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    errorMsg = null;
    try {
      const name = newName.trim();
      await api.createNetwork(name, newDriver);
      toast.success('Network created', `${name} (${newDriver})`);
      newName = '';
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create network.';
      errorMsg = msg;
      toast.error('Create failed', msg);
    }
  }

  async function handleRemove(n: NetworkSummaryDto) {
    const ok = await confirmDialog({
      title: 'Remove Network',
      message: 'Remove this Docker network?',
      targetName: `${n.name} (${n.short_id})`,
      confirmText: 'Remove Network',
      variant: 'danger',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      await api.removeNetwork(n.id);
      toast.success('Network removed', n.name);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to remove network.';
      errorMsg = msg;
      toast.error('Remove failed', msg);
    }
  }

  async function handlePrune() {
    const ok = await confirmDialog({
      title: 'Prune Unused Networks',
      message: 'Remove all custom Docker networks not used by any container?',
      confirmText: 'Prune Networks',
      variant: 'warning',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      const res = await api.pruneNetworks();
      toast.success('Networks pruned', `Removed ${res.deleted_count} unused network(s)`);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to prune networks.';
      errorMsg = msg;
      toast.error('Prune failed', msg);
    }
  }
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">
        {lang === 'id' ? 'Jaringan Docker' : 'Networks'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Kelola jaringan bridge/overlay Docker beserta kontainer yang terhubung.'
          : 'Inspect Docker networks, subnets, gateways, and connected containers.'}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder={lang === 'id' ? 'Cari jaringan…' : 'Search networks…'}
          bind:value={searchQuery}
          oninput={() => (page = 1)}
          class="pl-8 pr-3 py-1.5 w-44 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </div>
      {#if canOperate}
        <form onsubmit={handleCreate} class="flex items-center gap-2">
          <input
            type="text"
            placeholder="app_network"
            bind:value={newName}
            class="px-3 py-1.5 w-40 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs font-mono text-[var(--foreground)]"
          />
          <Select
            class="w-32"
            searchable={false}
            bind:value={newDriver}
            options={[
              { value: 'bridge', label: 'bridge' },
              { value: 'overlay', label: 'overlay' },
              { value: 'macvlan', label: 'macvlan' },
            ]}
          />
          <button
            type="submit"
            disabled={!newName.trim()}
            class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
          >
            <Plus class="w-3.5 h-3.5" /> Create
          </button>
        </form>
      {/if}
      {#if canDelete}
        <button
          type="button"
          onclick={handlePrune}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--warning-subtle)] border border-[var(--warning)]/40 text-[var(--warning)] text-xs font-medium inline-flex items-center gap-1.5 hover:opacity-90"
        >
          <Sparkles class="w-3.5 h-3.5" /> Prune
        </button>
      {/if}
      <button
        type="button"
        onclick={load}
        disabled={loading}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5"
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" /> Refresh
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
    {#if loading && networks.length === 0}
      <TableSkeleton rows={5} columns={5} />
    {:else if networks.length === 0}
      <div class="p-10 text-center space-y-2">
        <Network class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">No networks found.</div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th class="px-4 py-2.5">
              <SortHeader label="Name / ID" column="name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Driver / Scope" column="driver" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 font-medium">Subnet / Gateway</th>
            <th class="px-4 py-2.5 font-medium">Connected Containers</th>
            <th class="px-4 py-2.5 font-medium text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
          {#each networks as n}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5">
                <div class="font-medium text-[var(--foreground)]">{n.name}</div>
                <div class="font-mono text-[11px] text-[var(--muted-foreground)]">{n.short_id}</div>
              </td>
              <td class="px-4 py-2.5 text-[var(--muted-foreground)]">{n.driver} · {n.scope}</td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                {n.subnet ?? '—'} {n.gateway ? `(${n.gateway})` : ''}
              </td>
              <td class="px-4 py-2.5 text-[11px] text-[var(--muted-foreground)]">
                {n.connected_containers.join(', ') || '—'}
              </td>
              <td class="px-4 py-2.5 text-right">
                {#if canDelete && !['bridge', 'host', 'none'].includes(n.name)}
                  <button
                    type="button"
                    onclick={() => handleRemove(n)}
                    class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                    title="Remove Network"
                  >
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                {/if}
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
</div>
