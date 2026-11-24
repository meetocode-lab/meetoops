<script lang="ts">
  import { ChevronDown, Search, X } from 'lucide-svelte';
  import Checkbox from './Checkbox.svelte';

  export interface MultiSelectOption {
    value: string;
    label: string;
  }

  interface Props {
    options: MultiSelectOption[];
    values: string[];
    placeholder?: string;
    searchable?: boolean;
    class?: string;
    onchange?: (values: string[]) => void;
  }

  let {
    options,
    values = $bindable<string[]>([]),
    placeholder = 'Filter…',
    searchable = true,
    class: className = '',
    onchange,
  }: Props = $props();

  let open = $state(false);
  let query = $state('');
  let rootEl = $state<HTMLDivElement | null>(null);

  const filtered = $derived(
    !searchable || !query.trim()
      ? options
      : options.filter(
          (o) =>
            o.label.toLowerCase().includes(query.toLowerCase()) ||
            o.value.toLowerCase().includes(query.toLowerCase()),
        ),
  );

  function toggleValue(val: string) {
    const next = values.includes(val) ? values.filter((v) => v !== val) : [...values, val];
    values = next;
    onchange?.(next);
  }

  function clearAll(e: MouseEvent) {
    e.stopPropagation();
    values = [];
    onchange?.([]);
  }

  $effect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (rootEl && !rootEl.contains(e.target as Node)) open = false;
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') open = false;
    };
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', onKeyDown);
    };
  });
</script>

<div bind:this={rootEl} class="relative inline-block {className}">
  <button
    type="button"
    onclick={() => (open = !open)}
    class="w-full h-[34px] flex items-center justify-between gap-2 px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-left hover:bg-[var(--surface-elevated)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]/40"
  >
    <div class="flex items-center gap-1.5 min-w-0">
      <span class="truncate {values.length > 0 ? 'text-[var(--foreground)] font-medium' : 'text-[var(--muted-foreground)]'}">
        {placeholder}
      </span>
      {#if values.length > 0}
        <span class="px-1.5 py-0.2 rounded-full bg-[var(--primary)] text-[var(--primary-foreground)] text-[10px] font-semibold tabular-nums">
          {values.length}
        </span>
      {/if}
    </div>
    <div class="flex items-center gap-1 shrink-0">
      {#if values.length > 0}
        <span
          role="button"
          tabindex="0"
          onclick={clearAll}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              values = [];
              onchange?.([]);
            }
          }}
          class="p-0.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--muted-foreground)]"
          title="Clear"
        >
          <X class="w-3 h-3" />
        </span>
      {/if}
      <ChevronDown class="w-3.5 h-3.5 text-[var(--muted-foreground)] transition-transform {open ? 'rotate-180' : ''}" />
    </div>
  </button>

  {#if open}
    <div
      class="absolute z-50 mt-1.5 w-full min-w-[210px] rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] overflow-hidden"
      style="box-shadow: var(--shadow-sm);"
    >
      {#if searchable}
        <div class="p-1.5 border-b border-[var(--border)] bg-[var(--background)]">
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              bind:value={query}
              placeholder="Search…"
              class="w-full pl-8 pr-2.5 py-1 rounded bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
            />
          </div>
        </div>
      {/if}

      <div class="max-h-56 overflow-y-auto p-1.5 space-y-1">
        {#if filtered.length === 0}
          <div class="py-3 text-center text-xs text-[var(--muted-foreground)]">No options</div>
        {:else}
          {#each filtered as opt}
            <div class="px-2 py-1 rounded hover:bg-[var(--surface-elevated)]">
              <Checkbox
                checked={values.includes(opt.value)}
                label={opt.label}
                onchange={() => toggleValue(opt.value)}
                class="w-full"
              />
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>
