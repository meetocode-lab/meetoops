<script lang="ts">
  import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-svelte';
  import Select from './Select.svelte';

  interface Props {
    page: number;
    perPage: number;
    total: number;
    totalPages: number;
    onPageChange: (page: number) => void;
    onPerPageChange: (perPage: number) => void;
  }

  let { page, perPage, total, totalPages, onPageChange, onPerPageChange }: Props = $props();

  const startItem = $derived(total === 0 ? 0 : (page - 1) * perPage + 1);
  const endItem = $derived(Math.min(total, page * perPage));

  let perPageStr = $derived(String(perPage));
</script>

<div class="px-4 py-2.5 border-t border-[var(--border)] bg-[var(--surface)] rounded-b-[var(--radius-md)] flex flex-wrap items-center justify-between gap-3 text-xs text-[var(--muted-foreground)]">
  <div class="tabular-nums">
    Showing <span class="font-medium text-[var(--foreground)]">{startItem}–{endItem}</span> of
    <span class="font-medium text-[var(--foreground)]">{total}</span>
  </div>

  <div class="flex flex-wrap items-center gap-3">
    <div class="flex items-center gap-1.5">
      <span>Rows:</span>
      <Select
        class="w-20"
        placement="top"
        searchable={false}
        value={perPageStr}
        onchange={(v) => onPerPageChange(Number(v))}
        options={[
          { value: '10', label: '10' },
          { value: '25', label: '25' },
          { value: '50', label: '50' },
          { value: '100', label: '100' },
        ]}
      />
    </div>

    <div class="flex items-center gap-1 tabular-nums">
      <button
        type="button"
        disabled={page <= 1}
        onclick={() => onPageChange(1)}
        class="p-1.5 rounded border border-[var(--border-strong)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] disabled:opacity-40"
        title="First page"
      >
        <ChevronsLeft class="w-3.5 h-3.5" />
      </button>
      <button
        type="button"
        disabled={page <= 1}
        onclick={() => onPageChange(page - 1)}
        class="p-1.5 rounded border border-[var(--border-strong)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] disabled:opacity-40"
        title="Previous page"
      >
        <ChevronLeft class="w-3.5 h-3.5" />
      </button>
      <span class="px-2.5 font-mono text-[var(--foreground)]">
        {page} / {Math.max(1, totalPages)}
      </span>
      <button
        type="button"
        disabled={page >= totalPages}
        onclick={() => onPageChange(page + 1)}
        class="p-1.5 rounded border border-[var(--border-strong)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] disabled:opacity-40"
        title="Next page"
      >
        <ChevronRight class="w-3.5 h-3.5" />
      </button>
      <button
        type="button"
        disabled={page >= totalPages}
        onclick={() => onPageChange(totalPages)}
        class="p-1.5 rounded border border-[var(--border-strong)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] disabled:opacity-40"
        title="Last page"
      >
        <ChevronsRight class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</div>
