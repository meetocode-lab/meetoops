<script lang="ts">
  import { untrack } from 'svelte';
  import {
    api,
    setCsrfToken,
    type AuthSessionDto,
    type LocalDockerProbe,
    type RemoteTokenResponse,
    type SetupStage,
  } from '../lib/api';
  import { translations, type Language } from '../lib/i18n';
  import type { ThemePreference } from '../lib/theme';
  import ThemeToggle from './ThemeToggle.svelte';
  import Select from './ui/Select.svelte';
  import { CheckCircle2, ShieldCheck, Server, Terminal, AlertCircle, Globe, Loader2 } from 'lucide-svelte';

  interface Props {
    initialStage: SetupStage;
    lang: Language;
    theme: ThemePreference;
    onLangChange: (lang: Language) => void;
    onThemeChange: (theme: ThemePreference) => void;
    onComplete: (session: AuthSessionDto | null) => void;
  }

  let { initialStage, lang, theme, onLangChange, onThemeChange, onComplete }: Props = $props();
  const t = $derived(translations[lang]);

  function stageToStep(stage: SetupStage): number {
    if (stage === 'ADMIN_CREATED') return 3;
    if (stage === 'INFRASTRUCTURE_SETUP') return 4;
    return 1;
  }

  let step = $state(stageToStep(untrack(() => initialStage)));
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let createdSession = $state<AuthSessionDto | null>(null);

  // Step 2: Admin
  let fullName = $state('');
  let email = $state('');
  let username = $state('');
  let password = $state('');
  let confirmPassword = $state('');

  const passwordScore = $derived.by(() => {
    let score = 0;
    if (password.length >= 12) score++;
    if (/[A-Z]/.test(password)) score++;
    if (/[a-z]/.test(password)) score++;
    if (/[0-9]/.test(password)) score++;
    if (/[^A-Za-z0-9]/.test(password)) score++;
    return score;
  });

  // Step 3: Preferences
  let appName = $state('MeetoOps');
  let timezone = $state(Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC');
  let dateFormat = $state('YYYY-MM-DD');
  let timeFormat = $state('24h');
  let monitoringInterval = $state(5);

  // Step 4: Docker
  let dockerMode = $state<'local' | 'remote'>('local');
  let localProbe = $state<LocalDockerProbe | null>(null);
  let localServerName = $state('Local Docker');
  let localConnected = $state(false);
  let remoteServerName = $state('');
  let remoteGroup = $state('Production');
  let remoteResult = $state<RemoteTokenResponse | null>(null);

  async function handleCreateAdmin(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    if (password !== confirmPassword) {
      errorMsg = lang === 'id' ? 'Konfirmasi kata sandi tidak cocok.' : 'Passwords do not match.';
      return;
    }
    busy = true;
    try {
      const session = await api.setupCreateAdmin({
        full_name: fullName,
        email,
        username: username || undefined,
        password,
      });
      setCsrfToken(session.csrf_token);
      createdSession = session;
      step = 3;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to create administrator.';
    } finally {
      busy = false;
    }
  }

  async function handleSavePreferences(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    busy = true;
    try {
      await api.setupSavePreferences({
        app_name: appName,
        language: lang,
        timezone,
        date_format: dateFormat,
        time_format: timeFormat,
        monitoring_interval_secs: Number(monitoringInterval),
      });
      step = 4;
      await probeLocal();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to save preferences.';
    } finally {
      busy = false;
    }
  }

  async function probeLocal() {
    errorMsg = null;
    busy = true;
    try {
      localProbe = await api.setupTestLocalDocker();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to probe local Docker.';
    } finally {
      busy = false;
    }
  }

  async function connectLocal() {
    errorMsg = null;
    busy = true;
    try {
      await api.setupConnectLocalDocker(localServerName);
      localConnected = true;
      step = 5;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to connect local Docker.';
    } finally {
      busy = false;
    }
  }

  async function createRemote() {
    errorMsg = null;
    busy = true;
    try {
      remoteResult = await api.setupCreateRemoteToken(remoteServerName, remoteGroup);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to generate registration token.';
    } finally {
      busy = false;
    }
  }

  async function finishSetup() {
    errorMsg = null;
    busy = true;
    try {
      await api.setupComplete();
      onComplete(createdSession);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to finalize setup.';
    } finally {
      busy = false;
    }
  }

  const steps = $derived([
    { num: 1, label: t.setup.stepWelcome },
    { num: 2, label: t.setup.stepAdmin },
    { num: 3, label: t.setup.stepPrefs },
    { num: 4, label: t.setup.stepDocker },
    { num: 5, label: t.setup.stepComplete },
  ]);
</script>

<div class="min-h-screen bg-[var(--background)] flex flex-col items-center justify-center px-4 py-10">
  <div class="w-full max-w-[640px]">
    <!-- Top Brand & Language Toggle -->
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-2.5">
        <div class="w-8 h-8 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] flex items-center justify-center font-bold text-sm">
          M
        </div>
        <div>
          <div class="font-semibold text-[var(--foreground)] text-sm tracking-tight">MeetoOps</div>
          <div class="text-[11px] text-[var(--muted-foreground)]">{t.tagline}</div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <ThemeToggle {theme} onChange={onThemeChange} />
        <div class="flex items-center gap-1 bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-md)] p-0.5">
          <Globe class="w-3.5 h-3.5 text-[var(--muted-foreground)] ml-1.5" />
          <button
            type="button"
            class="px-2 py-1 text-xs rounded-[var(--radius-sm)] transition-colors {lang === 'en' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'}"
            onclick={() => onLangChange('en')}
          >
            EN
          </button>
          <button
            type="button"
            class="px-2 py-1 text-xs rounded-[var(--radius-sm)] transition-colors {lang === 'id' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'}"
            onclick={() => onLangChange('id')}
          >
            ID
          </button>
        </div>
      </div>
    </div>

    <!-- Stepper -->
    <div class="grid grid-cols-5 gap-2 mb-6">
      {#each steps as s}
        <div class="flex flex-col gap-1.5">
          <div
            class="h-1 rounded-full transition-colors {step >= s.num ? 'bg-[var(--primary)]' : 'bg-[var(--border)]'}"
          ></div>
          <span class="text-[11px] truncate {step === s.num ? 'text-[var(--foreground)] font-medium' : 'text-[var(--muted-foreground)]'}">
            {s.num}. {s.label}
          </span>
        </div>
      {/each}
    </div>

    <!-- Main Wizard Card -->
    <div class="bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-lg)] p-6">
      {#if errorMsg}
        <div class="mb-4 flex items-start gap-2.5 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
          <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
          <span>{errorMsg}</span>
        </div>
      {/if}

      {#if step === 1}
        <div class="space-y-5">
          <div>
            <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">{t.setup.title}</h1>
            <p class="text-[var(--muted-foreground)] mt-1.5">{t.setup.subtitle}</p>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-1">
            <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <ShieldCheck class="w-4 h-4 text-[var(--success)] mb-2" />
              <div class="text-xs font-medium text-[var(--foreground)]">SQLite WAL + AEAD</div>
              <div class="text-[11px] text-[var(--muted-foreground)] mt-0.5">Auto-initialized vault</div>
            </div>
            <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <Server class="w-4 h-4 text-[var(--primary)] mb-2" />
              <div class="text-xs font-medium text-[var(--foreground)]">Zero-Config</div>
              <div class="text-[11px] text-[var(--muted-foreground)] mt-0.5">No manual .env required</div>
            </div>
            <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <Terminal class="w-4 h-4 text-[var(--info)] mb-2" />
              <div class="text-xs font-medium text-[var(--foreground)]">Real Backend</div>
              <div class="text-[11px] text-[var(--muted-foreground)] mt-0.5">Rust + Axum + Bollard</div>
            </div>
          </div>

          <div class="flex justify-end pt-2">
            <button
              type="button"
              class="px-4 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs transition-colors"
              onclick={() => (step = 2)}
            >
              {t.setup.getStarted}
            </button>
          </div>
        </div>
      {:else if step === 2}
        <form onsubmit={handleCreateAdmin} class="space-y-4">
          <div>
            <h2 class="text-base font-semibold text-[var(--foreground)]">{t.setup.createAdminTitle}</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">{t.setup.createAdminDesc}</p>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.setup.fullName} *</span>
              <input
                type="text"
                required
                bind:value={fullName}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
              />
            </label>
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.setup.usernameOpt}</span>
              <input
                type="text"
                bind:value={username}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
              />
            </label>
          </div>

          <label class="block space-y-1">
            <span class="text-xs text-[var(--muted-foreground)]">{t.setup.email} *</span>
            <input
              type="email"
              required
              bind:value={email}
              class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
            />
          </label>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.setup.password} *</span>
              <input
                type="password"
                required
                minlength="12"
                bind:value={password}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
              />
            </label>
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.setup.confirmPassword} *</span>
              <input
                type="password"
                required
                minlength="12"
                bind:value={confirmPassword}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
              />
            </label>
          </div>

          <!-- Password Strength Bar -->
          <div class="space-y-1">
            <div class="grid grid-cols-5 gap-1">
              {#each [1, 2, 3, 4, 5] as lvl}
                <div
                  class="h-1 rounded-full {passwordScore >= lvl ? (passwordScore >= 4 ? 'bg-[var(--success)]' : 'bg-[var(--warning)]') : 'bg-[var(--border)]'}"
                ></div>
              {/each}
            </div>
            <p class="text-[11px] text-[var(--muted-foreground)]">{t.setup.passwordHint}</p>
          </div>

          <div class="flex justify-end pt-2">
            <button
              type="submit"
              disabled={busy}
              class="px-4 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50 inline-flex items-center gap-2"
            >
              {#if busy}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
              {t.setup.createAdminBtn}
            </button>
          </div>
        </form>
      {:else if step === 3}
        <form onsubmit={handleSavePreferences} class="space-y-4">
          <div>
            <h2 class="text-base font-semibold text-[var(--foreground)]">{t.setup.prefsTitle}</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">{t.setup.prefsDesc}</p>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.settings.appName}</span>
              <input
                type="text"
                bind:value={appName}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)]"
              />
            </label>
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.settings.timezone}</span>
              <input
                type="text"
                bind:value={timezone}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)]"
              />
            </label>
            <div class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)] block">{t.settings.dateFormat}</span>
              <Select
                class="w-full"
                searchable={false}
                bind:value={dateFormat}
                options={[
                  { value: 'YYYY-MM-DD', label: 'YYYY-MM-DD' },
                  { value: 'DD/MM/YYYY', label: 'DD/MM/YYYY' },
                  { value: 'MM/DD/YYYY', label: 'MM/DD/YYYY' },
                ]}
              />
            </div>
            <label class="block space-y-1">
              <span class="text-xs text-[var(--muted-foreground)]">{t.settings.monitoringInterval}</span>
              <input
                type="number"
                min="2"
                max="300"
                bind:value={monitoringInterval}
                class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] tabular-nums"
              />
            </label>
          </div>

          <div class="flex items-center justify-between pt-2">
            <button
              type="button"
              class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--surface-elevated)] border border-[var(--border)] text-xs text-[var(--foreground)]"
              onclick={() => {
                step = 4;
                probeLocal();
              }}
            >
              {t.setup.skipDefaults}
            </button>
            <button
              type="submit"
              disabled={busy}
              class="px-4 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50"
            >
              {t.setup.saveContinue}
            </button>
          </div>
        </form>
      {:else if step === 4}
        <div class="space-y-4">
          <div>
            <h2 class="text-base font-semibold text-[var(--foreground)]">{t.setup.dockerTitle}</h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-1">{t.setup.dockerDesc}</p>
          </div>

          <div class="flex gap-2">
            <button
              type="button"
              class="flex-1 py-2 px-3 rounded-[var(--radius-md)] border text-xs font-medium transition-colors {dockerMode === 'local' ? 'bg-[var(--primary-subtle)] border-[var(--primary)] text-[var(--primary)]' : 'bg-[var(--background)] border-[var(--border)] text-[var(--muted-foreground)]'}"
              onclick={() => (dockerMode = 'local')}
            >
              {t.setup.localDocker}
            </button>
            <button
              type="button"
              class="flex-1 py-2 px-3 rounded-[var(--radius-md)] border text-xs font-medium transition-colors {dockerMode === 'remote' ? 'bg-[var(--primary-subtle)] border-[var(--primary)] text-[var(--primary)]' : 'bg-[var(--background)] border-[var(--border)] text-[var(--muted-foreground)]'}"
              onclick={() => (dockerMode = 'remote')}
            >
              {t.setup.remoteAgent}
            </button>
          </div>

          {#if dockerMode === 'local'}
            <div class="p-4 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-xs font-medium text-[var(--foreground)]">Local Engine Probe</span>
                <button
                  type="button"
                  disabled={busy}
                  onclick={probeLocal}
                  class="px-2.5 py-1 rounded-[var(--radius-sm)] bg-[var(--surface-elevated)] border border-[var(--border)] text-xs text-[var(--foreground)]"
                >
                  {t.setup.testLocal}
                </button>
              </div>

              {#if localProbe}
                {#if localProbe.available}
                  <div class="text-xs text-[var(--success)] flex items-center gap-1.5">
                    <CheckCircle2 class="w-4 h-4" />
                    Connected ({localProbe.endpoint})
                  </div>
                  <div class="grid grid-cols-2 gap-2 text-xs text-[var(--muted-foreground)] tabular-nums">
                    <div>Docker: <span class="text-[var(--foreground)]">{localProbe.docker_version ?? '—'}</span></div>
                    <div>OS: <span class="text-[var(--foreground)]">{localProbe.operating_system ?? '—'}</span></div>
                    <div>Containers: <span class="text-[var(--foreground)]">{localProbe.containers_total ?? '—'}</span></div>
                    <div>Images: <span class="text-[var(--foreground)]">{localProbe.images_total ?? '—'}</span></div>
                  </div>
                  <button
                    type="button"
                    disabled={busy}
                    onclick={connectLocal}
                    class="w-full py-2 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-medium text-xs"
                  >
                    {t.setup.connectLocal}
                  </button>
                {:else}
                  <div class="text-xs text-[var(--warning)]">
                    Local Docker Engine not detected at <code class="font-mono">{localProbe.endpoint}</code>.
                  </div>
                  <p class="text-[11px] text-[var(--muted-foreground)] font-mono break-all">
                    {localProbe.error_message ?? 'Socket unavailable'}
                  </p>
                {/if}
              {/if}
            </div>
          {:else}
            <div class="p-4 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] space-y-3">
              <div class="grid grid-cols-2 gap-2">
                <input
                  type="text"
                  placeholder="prod-eu-01"
                  bind:value={remoteServerName}
                  class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)]"
                />
                <input
                  type="text"
                  placeholder="Production"
                  bind:value={remoteGroup}
                  class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)]"
                />
              </div>
              <button
                type="button"
                disabled={busy || remoteServerName.trim().length < 2}
                onclick={createRemote}
                class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50"
              >
                {t.setup.generateToken}
              </button>

              {#if remoteResult}
                <div class="space-y-2 pt-2 border-t border-[var(--border-subtle)]">
                  <div class="text-[11px] text-[var(--success)]">
                    Token valid until {remoteResult.expires_at}
                  </div>
                  <pre class="p-2.5 rounded-[var(--radius-sm)] bg-[var(--surface)] border border-[var(--border)] text-[11px] font-mono text-[var(--foreground)] overflow-x-auto whitespace-pre-wrap break-all">{remoteResult.install_command}</pre>
                </div>
              {/if}
            </div>
          {/if}

          <div class="flex justify-between pt-2">
            <button
              type="button"
              onclick={() => (step = 5)}
              class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--surface-elevated)] border border-[var(--border)] text-xs text-[var(--foreground)]"
            >
              {t.setup.skipForNow}
            </button>
            {#if remoteResult || localConnected}
              <button
                type="button"
                onclick={() => (step = 5)}
                class="px-4 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-medium text-xs"
              >
                {t.setup.saveContinue}
              </button>
            {/if}
          </div>
        </div>
      {:else}
        <div class="space-y-5">
          <div class="flex items-start gap-3">
            <CheckCircle2 class="w-6 h-6 text-[var(--success)] shrink-0 mt-0.5" />
            <div>
              <h2 class="text-base font-semibold text-[var(--foreground)]">{t.setup.completeTitle}</h2>
              <p class="text-xs text-[var(--muted-foreground)] mt-1">{t.setup.completeDesc}</p>
            </div>
          </div>

          <div class="space-y-2 text-xs">
            <div class="flex items-center justify-between p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <span class="text-[var(--muted-foreground)]">Super Administrator</span>
              <span class="text-[var(--success)] font-medium">Created</span>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <span class="text-[var(--muted-foreground)]">SQLite WAL + AES-256-GCM Vault</span>
              <span class="text-[var(--success)] font-medium">Active</span>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
              <span class="text-[var(--muted-foreground)]">Docker Server</span>
              <span class="text-[var(--foreground)]">
                {localConnected ? 'Local Connected' : remoteResult ? 'Remote Pending Agent' : 'Configure Later'}
              </span>
            </div>
          </div>

          <div class="flex justify-end">
            <button
              type="button"
              disabled={busy}
              onclick={finishSetup}
              class="px-5 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50"
            >
              {t.setup.goToDashboard}
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
