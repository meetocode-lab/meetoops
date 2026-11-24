<script lang="ts">
  import DatePicker from './DatePicker.svelte';
  import { X } from 'lucide-svelte';

  interface Props {
    from: string;
    to: string;
    onchange?: (from: string, to: string) => void;
  }

  let { from = $bindable(''), to = $bindable(''), onchange }: Props = $props();

  function clear() {
    from = '';
    to = '';
    onchange?.('', '');
  }
</script>

<div class="inline-flex items-center gap-1.5">
  <DatePicker
    class="w-36"
    placeholder="From date"
    bind:value={from}
    onchange={(v) => onchange?.(v, to)}
  />
  <span class="text-xs text-[var(--muted-foreground)]">–</span>
  <DatePicker
    class="w-36"
    placeholder="To date"
    bind:value={to}
    onchange={(v) => onchange?.(from, v)}
  />
  {#if from || to}
    <button
      type="button"
      onclick={clear}
      title="Clear date range"
      class="p-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
    >
      <X class="w-3.5 h-3.5" />
    </button>
  {/if}
</div>
