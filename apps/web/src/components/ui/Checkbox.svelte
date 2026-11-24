<script lang="ts">
  import { Check } from 'lucide-svelte';

  interface Props {
    checked: boolean;
    label?: string;
    disabled?: boolean;
    class?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    label = '',
    disabled = false,
    class: className = '',
    onchange,
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="checkbox"
  aria-checked={checked}
  {disabled}
  onclick={toggle}
  class="inline-flex items-center gap-2 text-xs text-[var(--foreground)] select-none focus:outline-none group disabled:opacity-50 disabled:pointer-events-none {className}"
>
  <span
    class="w-4 h-4 shrink-0 rounded-[4px] border flex items-center justify-center transition-colors group-focus-visible:ring-2 group-focus-visible:ring-[var(--ring)]/40 {checked ? 'bg-[var(--primary)] border-[var(--primary)] text-[var(--primary-foreground)]' : 'bg-[var(--surface)] border-[var(--border-strong)] group-hover:border-[var(--primary)]'}"
  >
    {#if checked}
      <Check class="w-3 h-3 stroke-[2.75]" />
    {/if}
  </span>
  {#if label}
    <span class="text-xs text-[var(--foreground)]">{label}</span>
  {/if}
</button>
