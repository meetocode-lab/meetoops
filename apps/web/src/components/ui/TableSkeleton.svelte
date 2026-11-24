<script lang="ts">
  import Skeleton from './Skeleton.svelte';

  interface Props {
    rows?: number;
    columns?: number;
  }

  let { rows = 5, columns = 5 }: Props = $props();
  const rowIndices = $derived(Array.from({ length: rows }, (_, i) => i));
  const colIndices = $derived(Array.from({ length: columns }, (_, i) => i));
</script>

<div class="w-full divide-y divide-[var(--border-subtle)]" aria-busy="true" aria-label="Loading table">
  <div class="px-4 py-3 bg-[var(--surface-secondary)] grid gap-4" style="grid-template-columns: repeat({columns}, minmax(0, 1fr));">
    {#each colIndices as _c}
      <Skeleton class="h-3 w-20 bg-[var(--border)]" />
    {/each}
  </div>
  {#each rowIndices as _r}
    <div class="px-4 py-3 grid gap-4 items-center" style="grid-template-columns: repeat({columns}, minmax(0, 1fr));">
      {#each colIndices as c}
        <Skeleton class="h-3.5 {c === 0 ? 'w-3/4' : c === columns - 1 ? 'w-16 ml-auto' : 'w-2/3'}" />
      {/each}
    </div>
  {/each}
</div>
