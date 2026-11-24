<script lang="ts">
  import { Calendar, ChevronLeft, ChevronRight } from 'lucide-svelte';

  interface Props {
    value: string; // YYYY-MM-DD
    placeholder?: string;
    disabled?: boolean;
    class?: string;
    onchange?: (value: string) => void;
  }

  let {
    value = $bindable(''),
    placeholder = 'Pick a date…',
    disabled = false,
    class: className = '',
    onchange,
  }: Props = $props();

  const weekdays = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
  let open = $state(false);
  let rootEl = $state<HTMLDivElement | null>(null);

  const initialDate = $derived(value ? new Date(`${value}T00:00:00`) : new Date());
  let viewYear = $state(new Date().getFullYear());
  let viewMonth = $state(new Date().getMonth());

  function toggle() {
    if (disabled) return;
    open = !open;
    if (open) {
      viewYear = initialDate.getFullYear();
      viewMonth = initialDate.getMonth();
    }
  }

  function prevMonth() {
    if (viewMonth === 0) {
      viewMonth = 11;
      viewYear -= 1;
    } else {
      viewMonth -= 1;
    }
  }

  function nextMonth() {
    if (viewMonth === 11) {
      viewMonth = 0;
      viewYear += 1;
    } else {
      viewMonth += 1;
    }
  }

  const monthLabel = $derived(
    new Date(viewYear, viewMonth, 1).toLocaleString(undefined, { month: 'long', year: 'numeric' }),
  );

  const days = $derived.by(() => {
    const firstDay = new Date(viewYear, viewMonth, 1).getDay();
    const totalDays = new Date(viewYear, viewMonth + 1, 0).getDate();
    const cells: Array<number | null> = [];
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= totalDays; d++) cells.push(d);
    return cells;
  });

  function formatIso(year: number, month: number, day: number): string {
    const mm = String(month + 1).padStart(2, '0');
    const dd = String(day).padStart(2, '0');
    return `${year}-${mm}-${dd}`;
  }

  function selectDay(day: number) {
    const iso = formatIso(viewYear, viewMonth, day);
    value = iso;
    onchange?.(iso);
    open = false;
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
    {disabled}
    onclick={toggle}
    class="w-full h-[34px] flex items-center justify-between gap-2 px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs transition-colors hover:bg-[var(--surface-elevated)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]/40 disabled:opacity-50"
  >
    <span class="font-mono tabular-nums {value ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'}">
      {value || placeholder}
    </span>
    <Calendar class="w-3.5 h-3.5 text-[var(--muted-foreground)] shrink-0" />
  </button>

  {#if open}
    <div
      class="absolute z-50 mt-1.5 w-64 p-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-2.5"
      style="box-shadow: var(--shadow-sm);"
    >
      <div class="flex items-center justify-between">
        <button
          type="button"
          onclick={prevMonth}
          class="p-1 rounded hover:bg-[var(--surface-elevated)] text-[var(--muted-foreground)]"
        >
          <ChevronLeft class="w-4 h-4" />
        </button>
        <span class="text-xs font-semibold text-[var(--foreground)]">{monthLabel}</span>
        <button
          type="button"
          onclick={nextMonth}
          class="p-1 rounded hover:bg-[var(--surface-elevated)] text-[var(--muted-foreground)]"
        >
          <ChevronRight class="w-4 h-4" />
        </button>
      </div>

      <div class="grid grid-cols-7 gap-1 text-center text-[10px] font-medium text-[var(--muted-foreground)]">
        {#each weekdays as wd}
          <div>{wd}</div>
        {/each}
      </div>

      <div class="grid grid-cols-7 gap-1 text-center text-xs tabular-nums">
        {#each days as d}
          {#if d === null}
            <div></div>
          {:else}
            {@const iso = formatIso(viewYear, viewMonth, d)}
            {@const isSelected = value === iso}
            <button
              type="button"
              onclick={() => selectDay(d)}
              class="h-7 w-7 mx-auto rounded-[var(--radius-sm)] transition-colors {isSelected ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-semibold' : 'text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
            >
              {d}
            </button>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</div>
