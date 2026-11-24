<script lang="ts">
  import { Sun, Moon, Monitor } from 'lucide-svelte';
  import type { ThemePreference } from '../lib/theme';

  interface Props {
    theme: ThemePreference;
    onChange: (next: ThemePreference) => void;
  }

  let { theme, onChange }: Props = $props();
</script>

<div
  class="inline-flex items-center gap-0.5 bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-md)] p-0.5"
  role="radiogroup"
  aria-label="Theme"
>
  <button
    type="button"
    title="Light"
    onclick={() => onChange('light')}
    class="p-1.5 rounded-[var(--radius-sm)] transition-colors {theme === 'light' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
  >
    <Sun class="w-3.5 h-3.5" />
  </button>
  <button
    type="button"
    title="System"
    onclick={() => onChange('system')}
    class="p-1.5 rounded-[var(--radius-sm)] transition-colors {theme === 'system' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
  >
    <Monitor class="w-3.5 h-3.5" />
  </button>
  <button
    type="button"
    title="Dark"
    onclick={() => onChange('dark')}
    class="p-1.5 rounded-[var(--radius-sm)] transition-colors {theme === 'dark' ? 'bg-[var(--primary)] text-[var(--primary-foreground)]' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
  >
    <Moon class="w-3.5 h-3.5" />
  </button>
</div>
