<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Icon as LucideIcon } from 'lucide-svelte';

  export interface ExplorerTabItem {
    id: string;
    label: string;
    icon?: typeof LucideIcon;
    badge?: string | number;
    badgeTone?: 'neutral' | 'primary' | 'success' | 'warning' | 'danger';
    dot?: 'success' | 'warning' | 'danger' | 'primary';
    disabled?: boolean;
  }

  let {
    tabs,
    active = $bindable(),
    onChange,
    actions,
    children,
    bodyClass = '',
    framed = true,
    compact = false,
  }: {
    tabs: ExplorerTabItem[];
    active: string;
    onChange?: (id: string) => void;
    actions?: Snippet;
    children?: Snippet;
    bodyClass?: string;
    framed?: boolean;
    compact?: boolean;
  } = $props();

  function selectTab(id: string) {
    if (active === id) return;
    active = id;
    onChange?.(id);
  }

  function handleKeydown(e: KeyboardEvent, index: number) {
    const enabledTabs = tabs.filter((t) => !t.disabled);
    if (enabledTabs.length <= 1) return;

    let nextIdx = -1;
    if (e.key === 'ArrowRight') {
      nextIdx = (index + 1) % tabs.length;
    } else if (e.key === 'ArrowLeft') {
      nextIdx = (index - 1 + tabs.length) % tabs.length;
    } else if (e.key === 'Home') {
      nextIdx = 0;
    } else if (e.key === 'End') {
      nextIdx = tabs.length - 1;
    }

    if (nextIdx >= 0 && !tabs[nextIdx]?.disabled) {
      e.preventDefault();
      selectTab(tabs[nextIdx].id);
    }
  }

  function badgeClasses(tone: ExplorerTabItem['badgeTone'], isActive: boolean): string {
    if (tone === 'success') return 'bg-[var(--success-subtle)] text-[var(--success)]';
    if (tone === 'warning') return 'bg-[var(--warning-subtle)] text-[var(--warning)]';
    if (tone === 'danger') return 'bg-[var(--danger-subtle)] text-[var(--danger)]';
    if (tone === 'primary') return 'bg-[var(--primary-subtle)] text-[var(--primary)]';
    return isActive
      ? 'bg-[var(--surface-secondary)] text-[var(--foreground)]'
      : 'bg-[var(--surface)]/70 text-[var(--muted-foreground)]';
  }

  function dotClasses(dot: ExplorerTabItem['dot']): string {
    if (dot === 'success') return 'bg-[var(--success)]';
    if (dot === 'warning') return 'bg-[var(--warning)]';
    if (dot === 'danger') return 'bg-[var(--danger)]';
    return 'bg-[var(--primary)]';
  }

  let activeIndex = $derived(tabs.findIndex((t) => t.id === active));
</script>

{#if framed}
  <div
    class="rounded-[var(--radius-lg)] bg-[var(--surface)] border border-[var(--border)] shadow-[var(--shadow-sm)] overflow-hidden"
  >
    <!-- File-Explorer Tab Chrome Strip -->
    <div
      class="flex items-end justify-between gap-2 bg-[var(--tab-track)] border-b border-[var(--border)] px-2 {compact ? 'pt-1.5' : 'pt-2'} select-none overflow-x-auto"
    >
      <div class="flex items-end min-w-0" role="tablist">
        {#each tabs as item, i (item.id)}
          {@const isActive = active === item.id}
          {@const Icon = item.icon}
          {@const showDivider = i > 0 && !isActive && activeIndex !== i - 1}

          {#if showDivider}
            <span
              class="self-center w-px h-3.5 bg-[var(--border-strong)]/70 mx-0.5 shrink-0"
              aria-hidden="true"
            ></span>
          {/if}

          <button
            type="button"
            role="tab"
            aria-selected={isActive}
            tabindex={isActive ? 0 : -1}
            disabled={item.disabled}
            onclick={() => selectTab(item.id)}
            onkeydown={(e) => handleKeydown(e, i)}
            class="relative inline-flex items-center gap-2 whitespace-nowrap transition-colors duration-150 cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed {compact
              ? 'px-3 py-1.5 text-[11.5px]'
              : 'px-4 py-2 text-xs'} {isActive
              ? 'explorer-tab-active z-10 -mb-px rounded-t-[9px] bg-[var(--surface)] text-[var(--foreground)] font-semibold border-t border-x border-[var(--border)] shadow-[0_-1px_2px_rgba(15,23,42,0.03)]'
              : 'mb-0.5 rounded-t-[8px] text-[var(--muted-foreground)] font-medium hover:text-[var(--foreground)] hover:bg-[var(--surface)]/55'}"
          >
            {#if isActive}
              <span
                class="absolute top-0 left-3 right-3 h-[2px] rounded-b-full bg-[var(--primary)]"
                aria-hidden="true"
              ></span>
            {/if}

            {#if Icon}
              <Icon
                class="w-3.5 h-3.5 shrink-0 {isActive
                  ? 'text-[var(--primary)]'
                  : 'text-[var(--subtle-foreground)]'}"
              />
            {/if}

            <span>{item.label}</span>

            {#if item.dot}
              <span class="w-1.5 h-1.5 rounded-full shrink-0 {dotClasses(item.dot)}"></span>
            {/if}

            {#if item.badge !== undefined && item.badge !== ''}
              <span
                class="px-1.5 py-0.5 rounded-full text-[10px] font-mono leading-none tabular-nums {badgeClasses(
                  item.badgeTone,
                  isActive,
                )}"
              >
                {item.badge}
              </span>
            {/if}
          </button>
        {/each}
      </div>

      {#if actions}
        <div class="flex items-center gap-1.5 pb-1.5 pr-1 shrink-0">
          {@render actions()}
        </div>
      {/if}
    </div>

    <!-- Seamlessly Fused Tab Body -->
    {#if children}
      <div class={bodyClass}>
        {@render children()}
      </div>
    {/if}
  </div>
{:else}
  <!-- Standalone Chrome Strip (for embedding directly at the top of an existing panel) -->
  <div
    class="flex items-end justify-between gap-2 bg-[var(--tab-track)] border-b border-[var(--border)] px-2 {compact ? 'pt-1.5' : 'pt-2'} select-none overflow-x-auto"
  >
    <div class="flex items-end min-w-0" role="tablist">
      {#each tabs as item, i (item.id)}
        {@const isActive = active === item.id}
        {@const Icon = item.icon}
        {@const showDivider = i > 0 && !isActive && activeIndex !== i - 1}

        {#if showDivider}
          <span
            class="self-center w-px h-3.5 bg-[var(--border-strong)]/70 mx-0.5 shrink-0"
            aria-hidden="true"
          ></span>
        {/if}

        <button
          type="button"
          role="tab"
          aria-selected={isActive}
          tabindex={isActive ? 0 : -1}
          disabled={item.disabled}
          onclick={() => selectTab(item.id)}
          onkeydown={(e) => handleKeydown(e, i)}
          class="relative inline-flex items-center gap-2 whitespace-nowrap transition-colors duration-150 cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed {compact
            ? 'px-3 py-1.5 text-[11.5px]'
            : 'px-4 py-2 text-xs'} {isActive
            ? 'explorer-tab-active z-10 -mb-px rounded-t-[9px] bg-[var(--surface)] text-[var(--foreground)] font-semibold border-t border-x border-[var(--border)]'
            : 'mb-0.5 rounded-t-[8px] text-[var(--muted-foreground)] font-medium hover:text-[var(--foreground)] hover:bg-[var(--surface)]/55'}"
        >
          {#if isActive}
            <span
              class="absolute top-0 left-3 right-3 h-[2px] rounded-b-full bg-[var(--primary)]"
              aria-hidden="true"
            ></span>
          {/if}

          {#if Icon}
            <Icon
              class="w-3.5 h-3.5 shrink-0 {isActive
                ? 'text-[var(--primary)]'
                : 'text-[var(--subtle-foreground)]'}"
            />
          {/if}

          <span>{item.label}</span>

          {#if item.dot}
            <span class="w-1.5 h-1.5 rounded-full shrink-0 {dotClasses(item.dot)}"></span>
          {/if}

          {#if item.badge !== undefined && item.badge !== ''}
            <span
              class="px-1.5 py-0.5 rounded-full text-[10px] font-mono leading-none tabular-nums {badgeClasses(
                item.badgeTone,
                isActive,
              )}"
            >
              {item.badge}
            </span>
          {/if}
        </button>
      {/each}
    </div>

    {#if actions}
      <div class="flex items-center gap-1.5 pb-1.5 pr-1 shrink-0">
        {@render actions()}
      </div>
    {/if}
  </div>
{/if}
