<script lang="ts">
  import { api, setCsrfToken, type AuthSessionDto } from '../lib/api';
  import { translations, type Language } from '../lib/i18n';
  import type { ThemePreference } from '../lib/theme';
  import ThemeToggle from './ThemeToggle.svelte';
  import { AlertCircle, Globe, Loader2 } from 'lucide-svelte';

  interface Props {
    lang: Language;
    theme: ThemePreference;
    appName: string;
    onLangChange: (lang: Language) => void;
    onThemeChange: (theme: ThemePreference) => void;
    onLogin: (session: AuthSessionDto) => void;
  }

  let { lang, theme, appName, onLangChange, onThemeChange, onLogin }: Props = $props();
  const t = $derived(translations[lang]);

  let identifier = $state('');
  let password = $state('');
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    busy = true;
    try {
      const session = await api.login(identifier, password);
      setCsrfToken(session.csrf_token);
      onLogin(session);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Authentication failed.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="min-h-screen bg-[var(--background)] flex flex-col items-center justify-center px-4">
  <div class="w-full max-w-[400px] space-y-5">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2.5">
        <div class="w-8 h-8 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] flex items-center justify-center font-bold text-sm">
          M
        </div>
        <div>
          <div class="font-semibold text-[var(--foreground)] text-sm">{appName}</div>
          <div class="text-[11px] text-[var(--muted-foreground)]">{t.tagline}</div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <ThemeToggle {theme} onChange={onThemeChange} />
        <div class="flex items-center gap-1 bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-md)] p-0.5">
          <Globe class="w-3.5 h-3.5 text-[var(--muted-foreground)] ml-1.5" />
          <button
            type="button"
            class="px-2 py-0.5 text-xs rounded-[var(--radius-sm)] {lang === 'en' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)]'}"
            onclick={() => onLangChange('en')}
          >
            EN
          </button>
          <button
            type="button"
            class="px-2 py-0.5 text-xs rounded-[var(--radius-sm)] {lang === 'id' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)]'}"
            onclick={() => onLangChange('id')}
          >
            ID
          </button>
        </div>
      </div>
    </div>

    <form
      onsubmit={handleSubmit}
      class="bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-lg)] p-6 space-y-4"
    >
      <div>
        <h1 class="text-base font-semibold text-[var(--foreground)]">{t.login.title}</h1>
        <p class="text-xs text-[var(--muted-foreground)] mt-1">{t.login.subtitle}</p>
      </div>

      {#if errorMsg}
        <div class="flex items-start gap-2 p-2.5 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
          <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
          <span>{errorMsg}</span>
        </div>
      {/if}

      <label class="block space-y-1">
        <span class="text-xs text-[var(--muted-foreground)]">{t.login.identifier}</span>
        <input
          type="text"
          required
          autocomplete="username"
          bind:value={identifier}
          class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </label>

      <label class="block space-y-1">
        <span class="text-xs text-[var(--muted-foreground)]">{t.login.password}</span>
        <input
          type="password"
          required
          autocomplete="current-password"
          bind:value={password}
          class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
        />
      </label>

      <button
        type="submit"
        disabled={busy}
        class="w-full py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50 inline-flex items-center justify-center gap-2"
      >
        {#if busy}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
        {t.login.signIn}
      </button>
    </form>
  </div>
</div>
