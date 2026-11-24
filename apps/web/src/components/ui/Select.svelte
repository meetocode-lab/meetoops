<script lang="ts">
  import { Check, ChevronDown, Search } from 'lucide-svelte';

  export interface SelectOption {
    value: string;
    label: string;
    description?: string;
  }

  interface Props {
    options: SelectOption[];
    value: string;
    placeholder?: string;
    searchPlaceholder?: string;
    emptyText?: string;
    searchable?: boolean;
    disabled?: boolean;
    placement?: 'bottom' | 'top';
    class?: string;
    onchange?: (value: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    placeholder = 'Select an option…',
    searchPlaceholder = 'Search…',
    emptyText = 'No results found.',
    searchable = true,
    disabled = false,
    placement = 'bottom',
    class: className = '',
    onchange,
  }: Props = $props();

  let open = $state(false);
  let query = $state('');
  let rootEl = $state<HTMLDivElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);

  const selectedOption = $derived(options.find((o) => o.value === value) ?? null);

  const filteredOptions = $derived.by(() => {
    if (!searchable || !query.trim()) return options;
    const q = query.trim().toLowerCase();
    return options.filter(
      (o) =>
        o.label.toLowerCase().includes(q) ||
        o.value.toLowerCase().includes(q) ||
        (o.description?.toLowerCase().includes(q) ?? false),
    );
  });

  function toggle() {
    if (disabled) return;
    open = !open;
    if (open) {
      query = '';
      if (searchable) {
        queueMicrotask(() => searchInputEl?.focus());
      }
    }
  }

  function selectItem(opt: SelectOption) {
    value = opt.value;
    onchange?.(opt.value);
    open = false;
    query = '';
  }

  $effect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (rootEl && !rootEl.contains(e.target as Node)) {
        open = false;
      }
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
    {disabled}
    onclick={toggle}
    aria-haspopup="listbox"
    aria-expanded={open}
    class="w-full h-[34px] flex items-center justify-between gap-2 px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-left transition-colors hover:bg-[var(--surface-elevated)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]/40 focus:border-[var(--ring)] disabled:opacity-50 disabled:pointer-events-none"
  >
    <span class="truncate {selectedOption ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'}">
      {selectedOption ? selectedOption.label : placeholder}
    </span>
    <ChevronDown class="w-3.5 h-3.5 shrink-0 text-[var(--muted-foreground)] transition-transform duration-150 {open ? 'rotate-180' : ''}" />
  </button>

  {#if open}
    <div
      role="listbox"
      class="absolute z-50 {placement === 'top' ? 'bottom-full mb-1.5' : 'top-full mt-1.5'} w-full min-w-[160px] rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] shadow-lg overflow-hidden"
      style="box-shadow: var(--shadow-sm);"
    >
      {#if searchable}
        <div class="p-1.5 border-b border-[var(--border)] bg-[var(--background)]">
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              bind:this={searchInputEl}
              type="text"
              bind:value={query}
              placeholder={searchPlaceholder}
              class="w-full pl-8 pr-2.5 py-1.5 rounded-[var(--radius-sm)] bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)] placeholder:text-[var(--subtle-foreground)] focus:outline-none focus:border-[var(--ring)]"
            />
          </div>
        </div>
      {/if}

      <div class="max-h-60 overflow-y-auto p-1 space-y-0.5">
        {#if filteredOptions.length === 0}
          <div class="px-2.5 py-4 text-center text-xs text-[var(--muted-foreground)]">
            {emptyText}
          </div>
        {:else}
          {#each filteredOptions as opt}
            {@const isSelected = opt.value === value}
            <button
              type="button"
              role="option"
              aria-selected={isSelected}
              onclick={() => selectItem(opt)}
              class="w-full flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-[var(--radius-sm)] text-xs text-left transition-colors {isSelected ? 'bg-[var(--primary-subtle)] text-[var(--primary)] font-medium' : 'text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
            >
              <div class="min-w-0 flex-1">
                <div class="truncate">{opt.label}</div>
                {#if opt.description}
                  <div class="text-[11px] text-[var(--muted-foreground)] truncate">{opt.description}</div>
                {/if}
              </div>
              {#if isSelected}
                <Check class="w-3.5 h-3.5 shrink-0 text-[var(--primary)]" />
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>
