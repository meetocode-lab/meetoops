<script lang="ts">
  import { translations, type Language } from '../lib/i18n';
  import { Layers } from 'lucide-svelte';

  interface Props {
    title: string;
    phase: string;
    description: string;
    lang: Language;
  }

  let { title, phase, description, lang }: Props = $props();
  const t = $derived(translations[lang]);
</script>

<div class="w-full p-6 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
  <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-[var(--info-subtle)] text-[var(--info)] text-[11px] font-medium">
    <Layers class="w-3.5 h-3.5" />
    {phase} · {t.comingPhase}
  </div>
  <h1 class="text-lg font-semibold text-[var(--foreground)]">{title}</h1>
  <p class="text-xs text-[var(--muted-foreground)] leading-relaxed">{description}</p>
</div>
