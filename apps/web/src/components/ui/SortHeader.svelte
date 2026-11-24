<script lang="ts">
  import { ArrowUpDown, ArrowUp, ArrowDown } from 'lucide-svelte';

  interface Props {
    label: string;
    column: string;
    activeColumn: string;
    direction: 'asc' | 'desc';
    align?: 'left' | 'right';
    onSort: (column: string) => void;
  }

  let { label, column, activeColumn, direction, align = 'left', onSort }: Props = $props();
  const isActive = $derived(activeColumn === column);
</script>

<button
  type="button"
  onclick={() => onSort(column)}
  class="inline-flex items-center gap-1 uppercase tracking-wider text-[11px] font-medium transition-colors hover:text-[var(--foreground)] {isActive ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)]'} {align === 'right' ? 'ml-auto' : ''}"
>
  <span>{label}</span>
  {#if isActive && direction === 'asc'}
    <ArrowUp class="w-3 h-3" />
  {:else if isActive && direction === 'desc'}
    <ArrowDown class="w-3 h-3" />
  {:else}
    <ArrowUpDown class="w-3 h-3 opacity-50" />
  {/if}
</button>
