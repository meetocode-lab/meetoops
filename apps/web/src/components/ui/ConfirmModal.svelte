<script lang="ts">
  import { AlertTriangle, X } from 'lucide-svelte';
  import { confirmStore, resolveConfirm } from '../../lib/confirm.svelte';

  function handleKeydown(e: KeyboardEvent) {
    if (!confirmStore.current) return;
    if (e.key === 'Escape') {
      resolveConfirm(false);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if confirmStore.current}
  {@const cfg = confirmStore.current}
  <div
    class="fixed inset-0 z-[110] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
  >
    <div class="w-full max-w-md rounded-2xl border border-[var(--border-strong)] bg-[var(--surface)] p-6 shadow-2xl">
      <div class="flex items-start justify-between gap-3">
        <div class="flex items-center gap-3">
          <div
            class={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl ${
              cfg.variant === 'warning'
                ? 'bg-[var(--warning-subtle)] text-[var(--warning)]'
                : cfg.variant === 'primary'
                  ? 'bg-[var(--primary-subtle)] text-[var(--primary)]'
                  : 'bg-[var(--danger-subtle)] text-[var(--danger)]'
            }`}
          >
            <AlertTriangle class="h-5 w-5" />
          </div>
          <h3 class="text-base font-semibold text-[var(--foreground)]">{cfg.title}</h3>
        </div>
        <button
          type="button"
          onclick={() => resolveConfirm(false)}
          class="rounded-lg p-1 text-[var(--muted-foreground)] hover:bg-[var(--surface-elevated)] hover:text-[var(--foreground)] transition-colors"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <p class="mt-3 text-sm leading-relaxed text-[var(--muted-foreground)]">{cfg.message}</p>

      {#if cfg.targetName}
        <div class="mt-3 rounded-lg border border-[var(--border)] bg-[var(--surface-secondary)] px-3 py-2 font-mono text-xs text-[var(--foreground)]">
          {cfg.targetName}
        </div>
      {/if}

      <div class="mt-6 flex items-center justify-end gap-2.5">
        <button
          type="button"
          onclick={() => resolveConfirm(false)}
          class="rounded-lg border border-[var(--border-strong)] bg-[var(--surface)] px-4 py-2 text-sm font-medium text-[var(--foreground)] transition-colors hover:bg-[var(--surface-elevated)]"
        >
          {cfg.cancelText ?? 'Cancel'}
        </button>
        <button
          type="button"
          onclick={() => resolveConfirm(true)}
          class={`rounded-lg px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 ${
            cfg.variant === 'warning'
              ? 'bg-[var(--warning)]'
              : cfg.variant === 'primary'
                ? 'bg-[var(--primary)]'
                : 'bg-[var(--danger)]'
          }`}
        >
          {cfg.confirmText ?? 'Confirm'}
        </button>
      </div>
    </div>
  </div>
{/if}
