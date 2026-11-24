<script lang="ts">
  import { api, type ImageSummaryDto, type PaginationMeta } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import { RefreshCw, Download, Trash2, AlertCircle, Layers, Loader2, Search, Sparkles } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
    canDelete: boolean;
  }

  let { lang, canOperate, canDelete }: Props = $props();

  let images = $state<ImageSummaryDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let pulling = $state(false);
  let pullRef = $state('');
  let searchQuery = $state('');
  let debouncedSearch = $state('');
  let sortBy = $state('created');
  let sortDir = $state<'asc' | 'desc'>('desc');
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
      const res = await api.queryImages({
        search: debouncedSearch || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      images = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load images.';
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

  async function handlePull(e: SubmitEvent) {
    e.preventDefault();
    if (!pullRef.trim()) return;
    pulling = true;
    errorMsg = null;
    try {
      const target = pullRef.trim();
      await api.pullImage(target);
      toast.success('Image pulled', target);
      pullRef = '';
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to pull image.';
      errorMsg = msg;
      toast.error('Pull failed', msg);
    } finally {
      pulling = false;
    }
  }

  async function handleRemove(img: ImageSummaryDto) {
    const label = img.repo_tags[0] ?? img.short_id;
    const ok = await confirmDialog({
      title: 'Remove Image',
      message: 'Remove this Docker image from the local host?',
      targetName: label,
      confirmText: 'Remove Image',
      variant: 'danger',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      await api.removeImage(img.id, false);
      toast.success('Image removed', label);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to remove image.';
      errorMsg = msg;
      toast.error('Remove failed', msg);
    }
  }

  function formatSize(bytes: number): string {
    const mb = bytes / 1024 ** 2;
    return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`;
  }

  async function handlePrune() {
    const ok = await confirmDialog({
      title: 'Prune Unused Images',
      message: 'Remove all dangling/unused Docker images to reclaim disk space?',
      confirmText: 'Prune Images',
      variant: 'warning',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      const res = await api.pruneImages();
      toast.success(
        'Images pruned',
        `Removed ${res.deleted_count} image(s) · Reclaimed ${formatSize(res.space_reclaimed_bytes)}`,
      );
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to prune images.';
      errorMsg = msg;
      toast.error('Prune failed', msg);
    }
  }
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">
        {lang === 'id' ? 'Image Docker' : 'Images'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Kelola image lokal, pull image dari registry, dan bersihkan image tak terpakai.'
          : 'Inspect local images, pull from registries, and remove unused images.'}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder={lang === 'id' ? 'Cari tag, ID…' : 'Search tag, ID…'}
          bind:value={searchQuery}
          oninput={() => (page = 1)}
          class="pl-8 pr-3 py-1.5 w-48 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </div>
      {#if canOperate}
        <form onsubmit={handlePull} class="flex items-center gap-2">
          <input
            type="text"
            placeholder="nginx:alpine"
            bind:value={pullRef}
            class="px-3 py-1.5 w-44 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs font-mono text-[var(--foreground)]"
          />
          <button
            type="submit"
            disabled={pulling || !pullRef.trim()}
            class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
          >
            {#if pulling}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Download class="w-3.5 h-3.5" />{/if}
            Pull
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
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        {lang === 'id' ? 'Muat Ulang' : 'Refresh'}
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
    {#if loading && images.length === 0}
      <TableSkeleton rows={5} columns={4} />
    {:else if images.length === 0}
      <div class="p-10 text-center space-y-2">
        <Layers class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">No images available.</div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th class="px-4 py-2.5">
              <SortHeader label="Tags" column="tag" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 font-medium">Image ID</th>
            <th class="px-4 py-2.5">
              <SortHeader label="Size" column="size" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 font-medium text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
          {#each images as img}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5 font-mono text-[var(--foreground)]">
                {img.repo_tags.length > 0 ? img.repo_tags.join(', ') : '<none>'}
              </td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                {img.short_id}
              </td>
              <td class="px-4 py-2.5 font-mono text-[var(--muted-foreground)] tabular-nums">
                {formatSize(img.size_bytes)}
              </td>
              <td class="px-4 py-2.5 text-right">
                {#if canDelete}
                  <button
                    type="button"
                    onclick={() => handleRemove(img)}
                    class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                    title="Remove Image"
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
