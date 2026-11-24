<script lang="ts">
  import { CheckCircle2, AlertTriangle, XCircle, Info, X } from 'lucide-svelte';
  import { toast } from '../../lib/toast.svelte';

  function toneClasses(variant: string) {
    switch (variant) {
      case 'success':
        return 'border-[var(--success)]/45 bg-[var(--surface)] text-[var(--foreground)]';
      case 'error':
        return 'border-[var(--danger)]/45 bg-[var(--surface)] text-[var(--foreground)]';
      case 'warning':
        return 'border-[var(--warning)]/45 bg-[var(--surface)] text-[var(--foreground)]';
      default:
        return 'border-[var(--primary)]/45 bg-[var(--surface)] text-[var(--foreground)]';
    }
  }
</script>

{#if toast.items.length > 0}
  <div
    class="pointer-events-none fixed bottom-5 right-5 z-[100] flex w-full max-w-sm flex-col gap-2.5"
    role="status"
    aria-live="polite"
  >
    {#each toast.items as item (item.id)}
      <div
        class={`pointer-events-auto flex items-start gap-3 rounded-xl border px-4 py-3 shadow-xl transition-all ${toneClasses(item.variant)}`}
      >
        <div class="mt-0.5 shrink-0">
          {#if item.variant === 'success'}
            <CheckCircle2 class="h-4 w-4 text-[var(--success)]" />
          {:else if item.variant === 'error'}
            <XCircle class="h-4 w-4 text-[var(--danger)]" />
          {:else if item.variant === 'warning'}
            <AlertTriangle class="h-4 w-4 text-[var(--warning)]" />
          {:else}
            <Info class="h-4 w-4 text-[var(--primary)]" />
          {/if}
        </div>
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium leading-snug text-[var(--foreground)]">{item.title}</p>
          {#if item.description}
            <p class="mt-0.5 text-xs leading-relaxed text-[var(--muted-foreground)]">{item.description}</p>
          {/if}
        </div>
        <button
          type="button"
          onclick={() => toast.dismiss(item.id)}
          class="rounded-md p-1 text-[var(--muted-foreground)] transition-colors hover:bg-[var(--surface-elevated)] hover:text-[var(--foreground)]"
          aria-label="Dismiss notification"
        >
          <X class="h-3.5 w-3.5" />
        </button>
      </div>
    {/each}
  </div>
{/if}
