<script lang="ts">
  interface Props {
    class?: string;
  }

  let { class: className = 'h-4 w-full' }: Props = $props();
</script>

<div
  aria-hidden="true"
  class="animate-pulse rounded-[var(--radius-sm)] bg-[var(--surface-secondary)] {className}"
></div>
