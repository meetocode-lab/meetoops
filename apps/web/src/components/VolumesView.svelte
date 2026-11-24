<script lang="ts">
  import { api, type PaginationMeta, type VolumeSummaryDto } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import { RefreshCw, Plus, Trash2, AlertCircle, HardDrive, Search, Sparkles } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
    canDelete: boolean;
  }

  let { lang, canOperate, canDelete }: Props = $props();

  let volumes = $state<VolumeSummaryDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let newName = $state('');
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
      const res = await api.queryVolumes({
        search: debouncedSearch || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      volumes = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load volumes.';
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
      await api.createVolume(name);
      toast.success('Volume created', name);
      newName = '';
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create volume.';
      errorMsg = msg;
      toast.error('Create failed', msg);
    }
  }

  async function handleRemove(v: VolumeSummaryDto) {
    const ok = await confirmDialog({
      title: 'Remove Volume',
      message: 'All data stored in this Docker volume will be permanently lost.',
      targetName: v.name,
      confirmText: 'Remove Volume',
      variant: 'danger',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      await api.removeVolume(v.name, false);
      toast.success('Volume removed', v.name);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to remove volume.';
      errorMsg = msg;
      toast.error('Remove failed', msg);
    }
  }

  async function handlePrune() {
    const ok = await confirmDialog({
      title: 'Prune Unused Volumes',
      message: 'Permanently delete all local Docker volumes not referenced by any container?',
      confirmText: 'Prune Volumes',
      variant: 'danger',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      const res = await api.pruneVolumes();
      const mb = (res.space_reclaimed_bytes / 1024 ** 2).toFixed(1);
      toast.success('Volumes pruned', `Removed ${res.deleted_count} volume(s) · Reclaimed ${mb} MB`);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to prune volumes.';
      errorMsg = msg;
      toast.error('Prune failed', msg);
    }
  }
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">Volumes</h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Kelola volume persisten Docker dan lihat kontainer yang memakainya.'
          : 'Manage persistent Docker volumes and inspect container mount references.'}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder={lang === 'id' ? 'Cari volume…' : 'Search volumes…'}
          bind:value={searchQuery}
          oninput={() => (page = 1)}
          class="pl-8 pr-3 py-1.5 w-44 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </div>
      {#if canOperate}
        <form onsubmit={handleCreate} class="flex items-center gap-2">
          <input
            type="text"
            placeholder="my_data_volume"
            bind:value={newName}
            class="px-3 py-1.5 w-44 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs font-mono text-[var(--foreground)]"
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
    {#if loading && volumes.length === 0}
      <TableSkeleton rows={5} columns={5} />
    {:else if volumes.length === 0}
      <div class="p-10 text-center space-y-2">
        <HardDrive class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">No volumes found.</div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th class="px-4 py-2.5">
              <SortHeader label="Name" column="name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Driver" column="driver" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 font-medium">Used By</th>
            <th class="px-4 py-2.5 font-medium">Mountpoint</th>
            <th class="px-4 py-2.5 font-medium text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
          {#each volumes as v}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5 font-mono font-medium text-[var(--foreground)]">{v.name}</td>
              <td class="px-4 py-2.5 text-[var(--muted-foreground)]">{v.driver}</td>
              <td class="px-4 py-2.5">
                {#if v.used_by.length > 0}
                  <span class="px-2 py-0.5 rounded bg-[var(--success-subtle)] text-[var(--success)] text-[11px]">
                    {v.used_by.join(', ')}
                  </span>
                {:else}
                  <span class="text-[var(--muted-foreground)] text-[11px]">Unused</span>
                {/if}
              </td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] max-w-[300px] truncate">
                {v.mountpoint}
              </td>
              <td class="px-4 py-2.5 text-right">
                {#if canDelete}
                  <button
                    type="button"
                    onclick={() => handleRemove(v)}
                    class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                    title="Remove Volume"
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
