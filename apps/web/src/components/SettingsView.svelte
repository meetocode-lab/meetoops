<script lang="ts">
  import {
    api,
    type ApiTokenDto,
    type AuditLogEntry,
    type PaginationMeta,
    type PortCheckResult,
    type Role,
    type SettingsBackupBundle,
    type SettingsResponseDto,
    type SnapshotInfoDto,
    type UserPublic,
  } from '../lib/api';
  import { translations, type Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import DateRangePicker from './ui/DateRangePicker.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import ExplorerTabs from './ui/ExplorerTabs.svelte';
  import { ShieldCheck, CheckCircle2, AlertCircle, Trash2, Plus, Radio, Search, KeyRound, Download, Upload, DatabaseBackup, Copy, Check, SlidersHorizontal, Users, ScrollText } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canWriteSettings: boolean;
    onLangChange: (lang: Language) => void;
    onAppNameChange: (name: string) => void;
  }

  let { lang, canWriteSettings, onLangChange, onAppNameChange }: Props = $props();
  const t = $derived(translations[lang]);

  let tab = $state<'general' | 'users' | 'security' | 'backup' | 'audit'>('general');
  let apiTokens = $state<ApiTokenDto[]>([]);
  let newTokenName = $state('');
  let createdTokenPlain = $state<string | null>(null);
  let copiedToken = $state(false);
  let snapshots = $state<SnapshotInfoDto[]>([]);
  let importJsonText = $state('');
  let data = $state<SettingsResponseDto | null>(null);
  let users = $state<UserPublic[]>([]);
  let usersMeta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let userSearch = $state('');
  let userRoles = $state<string[]>([]);
  let userSortBy = $state('created_at');
  let userSortDir = $state<'asc' | 'desc'>('asc');
  let userPage = $state(1);
  let userPerPage = $state(10);

  let auditLogs = $state<AuditLogEntry[]>([]);
  let auditMeta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let auditSearch = $state('');
  let auditStatuses = $state<string[]>([]);
  let auditResources = $state<string[]>([]);
  let auditFrom = $state('');
  let auditTo = $state('');
  let auditSortBy = $state('created_at');
  let auditSortDir = $state<'asc' | 'desc'>('desc');
  let auditPage = $state(1);
  let auditPerPage = $state(10);

  let portCheck = $state<PortCheckResult | null>(null);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let successMsg = $state<string | null>(null);

  // Add user form
  let newFullName = $state('');
  let newEmail = $state('');
  let newPassword = $state('');
  let newRole = $state<Role>('operator');

  async function loadSettings() {
    try {
      data = await api.getSettings();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load settings.';
    }
  }

  async function loadUsers() {
    try {
      const res = await api.queryUsers({
        search: userSearch || undefined,
        role: userRoles.join(',') || undefined,
        sort_by: userSortBy,
        sort_dir: userSortDir,
        page: userPage,
        per_page: userPerPage,
      });
      users = res.items;
      usersMeta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load users.';
    }
  }

  async function loadAudit() {
    try {
      const res = await api.queryAuditLogs({
        search: auditSearch || undefined,
        status: auditStatuses.join(',') || undefined,
        resource_type: auditResources.join(',') || undefined,
        from: auditFrom || undefined,
        to: auditTo || undefined,
        sort_by: auditSortBy,
        sort_dir: auditSortDir,
        page: auditPage,
        per_page: auditPerPage,
      });
      auditLogs = res.items;
      auditMeta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load audit logs.';
    }
  }

  function sortUsers(col: string) {
    if (userSortBy === col) userSortDir = userSortDir === 'asc' ? 'desc' : 'asc';
    else {
      userSortBy = col;
      userSortDir = 'asc';
    }
    userPage = 1;
  }

  function sortAudit(col: string) {
    if (auditSortBy === col) auditSortDir = auditSortDir === 'asc' ? 'desc' : 'asc';
    else {
      auditSortBy = col;
      auditSortDir = 'desc';
    }
    auditPage = 1;
  }

  async function loadTokens() {
    try {
      apiTokens = await api.listApiTokens();
    } catch {
      // Non-fatal
    }
  }

  async function handleCreateToken(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    try {
      const res = await api.createApiToken(newTokenName, 90);
      createdTokenPlain = res.plaintext_token;
      newTokenName = '';
      await loadTokens();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to create API token.';
    }
  }

  async function handleRevokeToken(id: string) {
    const ok = await confirmDialog({
      title: 'Revoke API Token',
      message: 'Any automation or CLI using this token will immediately lose access.',
      confirmText: 'Revoke Token',
      variant: 'danger',
    });
    if (!ok) return;
    await api.revokeApiToken(id);
    toast.success('API token revoked');
    await loadTokens();
  }

  async function loadSnapshots() {
    if (!canWriteSettings) return;
    try {
      snapshots = await api.listDbSnapshots();
    } catch {
      // Non-fatal
    }
  }

  async function handleCreateSnapshot() {
    busy = true;
    errorMsg = null;
    successMsg = null;
    try {
      const snap = await api.createDbSnapshot();
      successMsg = `Database snapshot created: ${snap.filename}`;
      await loadSnapshots();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to create database snapshot.';
    } finally {
      busy = false;
    }
  }

  async function handleExportSettings() {
    try {
      const bundle = await api.exportSettingsBackup();
      const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `meetoops-settings-${bundle.exported_at.slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to export settings.';
    }
  }

  async function handleImportSettings(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    successMsg = null;
    try {
      const parsed = JSON.parse(importJsonText) as SettingsBackupBundle;
      await api.importSettingsBackup(parsed);
      importJsonText = '';
      successMsg = 'Settings restored & verified (SHA-256 matched).';
      await loadSettings();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Invalid backup JSON or checksum mismatch.';
    }
  }

  $effect(() => {
    loadSettings();
  });

  $effect(() => {
    if (tab === 'security') loadTokens();
    if (tab === 'backup') loadSnapshots();
  });

  $effect(() => {
    void userSearch;
    void userRoles;
    void userSortBy;
    void userSortDir;
    void userPage;
    void userPerPage;
    if (tab === 'users') loadUsers();
  });

  $effect(() => {
    void auditSearch;
    void auditStatuses;
    void auditResources;
    void auditFrom;
    void auditTo;
    void auditSortBy;
    void auditSortDir;
    void auditPage;
    void auditPerPage;
    if (tab === 'audit') loadAudit();
  });

  async function handleCheckPort() {
    if (!data) return;
    errorMsg = null;
    busy = true;
    try {
      portCheck = await api.checkPort(Number(data.settings.listen_port));
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Port check failed.';
    } finally {
      busy = false;
    }
  }

  async function handleSaveSettings(e: SubmitEvent) {
    e.preventDefault();
    if (!data) return;
    errorMsg = null;
    successMsg = null;
    busy = true;
    try {
      const updated = await api.updateSettings({
        ...data.settings,
        listen_port: Number(data.settings.listen_port),
        monitoring_interval_secs: Number(data.settings.monitoring_interval_secs),
        session_timeout_hours: Number(data.settings.session_timeout_hours),
        max_login_attempts: Number(data.settings.max_login_attempts),
        audit_retention_days: Number(data.settings.audit_retention_days),
      });
      data = updated;
      if (updated.settings.language === 'en' || updated.settings.language === 'id') {
        onLangChange(updated.settings.language);
      }
      onAppNameChange(updated.settings.app_name);
      successMsg = t.settings.savedToast;
      portCheck = null;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to save settings.';
    } finally {
      busy = false;
    }
  }

  async function handleCreateUser(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    busy = true;
    try {
      await api.createUser({
        full_name: newFullName,
        email: newEmail,
        password: newPassword,
        role: newRole,
      });
      newFullName = '';
      newEmail = '';
      newPassword = '';
      await loadUsers();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to create user.';
    } finally {
      busy = false;
    }
  }

  async function handleDeleteUser(u: UserPublic) {
    const ok = await confirmDialog({
      title: 'Delete User',
      message: 'This permanently deletes the user account and revokes all active sessions.',
      targetName: `${u.full_name} (${u.email})`,
      confirmText: 'Delete User',
      variant: 'danger',
    });
    if (!ok) return;
    errorMsg = null;
    try {
      await api.deleteUser(u.id);
      toast.success('User deleted', u.email);
      await loadUsers();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to delete user.';
    }
  }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">{t.settings.title}</h1>
    <p class="text-xs text-[var(--muted-foreground)] mt-0.5">{t.settings.subtitle}</p>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}
  {#if successMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--success-subtle)] border border-[var(--success)]/40 text-[var(--success)] text-xs">
      <CheckCircle2 class="w-4 h-4 shrink-0" />
      <span>{successMsg}</span>
    </div>
  {/if}

  <ExplorerTabs
    bind:active={tab}
    bodyClass="p-5 space-y-4"
    tabs={[
      { id: 'general', label: t.settings.tabGeneral, icon: SlidersHorizontal },
      { id: 'users', label: t.settings.tabUsers, icon: Users },
      { id: 'security', label: t.settings.tabSecurity, icon: ShieldCheck },
      { id: 'backup', label: 'Backup & Restore', icon: DatabaseBackup },
      { id: 'audit', label: t.settings.tabAudit, icon: ScrollText },
    ]}
  >
  {#if tab === 'general' && data}
    <form onsubmit={handleSaveSettings} class="space-y-5 w-full">
      <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] grid grid-cols-1 sm:grid-cols-2 gap-4">
        <label class="block space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">{t.settings.appName}</span>
          <input
            type="text"
            disabled={!canWriteSettings}
            bind:value={data.settings.app_name}
            class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)]"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">{t.settings.appUrl}</span>
          <input
            type="url"
            disabled={!canWriteSettings}
            bind:value={data.settings.app_url}
            class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] font-mono text-xs"
          />
        </label>
        <div class="block space-y-1">
          <span class="text-xs text-[var(--muted-foreground)] block">{t.settings.language}</span>
          <Select
            class="w-full"
            searchable={false}
            disabled={!canWriteSettings}
            bind:value={data.settings.language}
            options={[
              { value: 'en', label: 'English (EN)' },
              { value: 'id', label: 'Bahasa Indonesia (ID)' },
            ]}
          />
        </div>
        <label class="block space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">{t.settings.timezone}</span>
          <input
            type="text"
            disabled={!canWriteSettings}
            bind:value={data.settings.timezone}
            class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)]"
          />
        </label>
        <label class="block space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">{t.settings.monitoringInterval}</span>
          <input
            type="number"
            min="2"
            max="300"
            disabled={!canWriteSettings}
            bind:value={data.settings.monitoring_interval_secs}
            class="w-full px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] tabular-nums"
          />
        </label>
      </div>

      <!-- Network Listen Port + Live Availability Probe -->
      <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="text-sm font-semibold text-[var(--foreground)] flex items-center gap-2">
              <Radio class="w-4 h-4 text-[var(--primary)]" />
              {t.settings.networkPortTitle}
            </h2>
            <p class="text-xs text-[var(--muted-foreground)] mt-0.5">{t.settings.networkPortDesc}</p>
          </div>
          <span class="px-2 py-1 rounded-[var(--radius-sm)] bg-[var(--background)] border border-[var(--border)] font-mono text-xs text-[var(--primary)] tabular-nums">
            {t.settings.activePort}: {data.active_port}
          </span>
        </div>

        <div class="flex flex-wrap items-end gap-3">
          <label class="space-y-1">
            <span class="text-xs text-[var(--muted-foreground)]">{t.settings.listenPort}</span>
            <input
              type="number"
              min="1024"
              max="65535"
              disabled={!canWriteSettings}
              bind:value={data.settings.listen_port}
              class="w-36 px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-[var(--foreground)] font-mono tabular-nums"
            />
          </label>
          <button
            type="button"
            disabled={!canWriteSettings || busy}
            onclick={handleCheckPort}
            class="px-3.5 py-2 rounded-[var(--radius-md)] bg-[var(--surface-elevated)] hover:border-[var(--muted-foreground)] border border-[var(--border)] text-xs text-[var(--foreground)] disabled:opacity-50"
          >
            {t.settings.checkPortBtn}
          </button>
        </div>

        {#if portCheck}
          <div class="p-3 rounded-[var(--radius-md)] border text-xs {portCheck.available ? 'bg-[var(--success-subtle)] border-[var(--success)]/40 text-[var(--success)]' : 'bg-[var(--danger-subtle)] border-[var(--danger)]/40 text-[var(--danger)]'}">
            <div>{portCheck.reason}</div>
            {#if !portCheck.available && portCheck.suggested_ports.length > 0}
              <div class="mt-2 flex items-center gap-2 flex-wrap">
                <span class="text-[var(--muted-foreground)]">Suggested free ports:</span>
                {#each portCheck.suggested_ports as sp}
                  <button
                    type="button"
                    onclick={() => {
                      if (data) data.settings.listen_port = sp;
                    }}
                    class="px-2 py-0.5 rounded bg-[var(--surface)] border border-[var(--border)] font-mono text-[var(--foreground)] hover:border-[var(--primary)]"
                  >
                    {sp}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        {#if data.restart_required_for_port}
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--warning-subtle)] border border-[var(--warning)]/40 text-[var(--warning)] text-xs">
            {t.settings.restartNotice}
          </div>
        {/if}
      </div>

      {#if canWriteSettings}
        <div class="flex justify-end">
          <button
            type="submit"
            disabled={busy}
            class="px-4 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] font-medium text-xs disabled:opacity-50"
          >
            {t.settings.saveSettings}
          </button>
        </div>
      {/if}
    </form>
  {:else if tab === 'users'}
    <div class="space-y-4 w-full">
      {#if canWriteSettings}
        <form onsubmit={handleCreateUser} class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] grid grid-cols-1 sm:grid-cols-5 gap-2.5 items-end">
          <input type="text" required placeholder="Full Name" bind:value={newFullName} class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-xs text-[var(--foreground)]" />
          <input type="email" required placeholder="user@example.com" bind:value={newEmail} class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-xs text-[var(--foreground)]" />
          <input type="password" required minlength="12" placeholder="Password (12+ chars)" bind:value={newPassword} class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)] text-xs text-[var(--foreground)]" />
          <Select
            class="w-full"
            searchable={false}
            bind:value={newRole}
            options={[
              { value: 'admin', label: 'Admin' },
              { value: 'operator', label: 'Operator' },
              { value: 'viewer', label: 'Viewer' },
            ]}
          />
          <button type="submit" disabled={busy} class="px-3 py-2 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-medium text-xs inline-flex items-center justify-center gap-1.5">
            <Plus class="w-3.5 h-3.5" />
            {t.settings.addUser}
          </button>
        </form>
      {/if}

      <!-- Server-Side Search & Role Filter Toolbar -->
      <div class="flex flex-wrap items-center gap-2">
        <div class="relative">
          <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Search name or email…"
            bind:value={userSearch}
            oninput={() => (userPage = 1)}
            class="pl-8 pr-3 py-1.5 w-60 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
          />
        </div>
        <MultiSelect
          class="w-44"
          searchable={false}
          placeholder="Filter Roles"
          bind:values={userRoles}
          onchange={() => (userPage = 1)}
          options={[
            { value: 'super_admin', label: 'Super Admin' },
            { value: 'admin', label: 'Admin' },
            { value: 'operator', label: 'Operator' },
            { value: 'viewer', label: 'Viewer' },
          ]}
        />
      </div>

      <div class="rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
              <th class="px-4 py-2.5">
                <SortHeader label="Name" column="full_name" activeColumn={userSortBy} direction={userSortDir} onSort={sortUsers} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Email" column="email" activeColumn={userSortBy} direction={userSortDir} onSort={sortUsers} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Role" column="role" activeColumn={userSortBy} direction={userSortDir} onSort={sortUsers} />
              </th>
              <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)] text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
            {#each users as u}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">{u.full_name}</td>
                <td class="px-4 py-2.5 font-mono text-[var(--muted-foreground)]">{u.email}</td>
                <td class="px-4 py-2.5">
                  <span class="px-2 py-0.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[11px]">{u.role}</span>
                </td>
                <td class="px-4 py-2.5 text-right">
                  {#if u.role !== 'super_admin'}
                    <button
                      type="button"
                      onclick={() => handleDeleteUser(u)}
                      class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                      title={t.delete}
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <PaginationBar
          page={usersMeta.page}
          perPage={usersMeta.per_page}
          total={usersMeta.total}
          totalPages={usersMeta.total_pages}
          onPageChange={(p) => (userPage = p)}
          onPerPageChange={(pp) => {
            userPerPage = pp;
            userPage = 1;
          }}
        />
      </div>
    </div>
  {:else if tab === 'security' && data}
    <div class="w-full space-y-5">
      <div class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-4">
        <div class="flex items-center gap-2.5 text-[var(--success)]">
          <ShieldCheck class="w-5 h-5" />
          <span class="text-sm font-semibold">{t.settings.vaultStatus}</span>
        </div>
        <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 text-xs">
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">{t.settings.vaultFingerprint}</div>
            <div class="font-mono text-[var(--foreground)] mt-1">{data.vault_fingerprint}…</div>
          </div>
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Password Hashing</div>
            <div class="font-mono text-[var(--foreground)] mt-1">Argon2id (OWASP)</div>
          </div>
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Session Timeout</div>
            <div class="font-mono text-[var(--foreground)] mt-1 tabular-nums">{data.settings.session_timeout_hours}h</div>
          </div>
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
            <div class="text-[var(--muted-foreground)]">Max Login Attempts</div>
            <div class="font-mono text-[var(--foreground)] mt-1 tabular-nums">{data.settings.max_login_attempts}</div>
          </div>
        </div>
      </div>

      <!-- Personal API Tokens (Bearer Auth) -->
      <div class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-4">
        <div class="flex items-center gap-2 text-sm font-semibold text-[var(--foreground)]">
          <KeyRound class="w-4 h-4 text-[var(--primary)]" />
          Personal API Tokens (`Authorization: Bearer mto_...`)
        </div>

        <form onsubmit={handleCreateToken} class="flex flex-wrap gap-2">
          <input
            type="text"
            required
            placeholder="prometheus-scraper or ci-deploy"
            bind:value={newTokenName}
            class="px-3 py-1.5 w-64 rounded bg-[var(--background)] border border-[var(--border-strong)] text-xs"
          />
          <button type="submit" class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5">
            <Plus class="w-3.5 h-3.5" /> Generate Token (90d)
          </button>
        </form>

        {#if createdTokenPlain}
          <div class="p-3 rounded bg-[var(--warning-subtle)] border border-[var(--warning)]/50 space-y-1.5 text-xs">
            <div class="flex items-center justify-between">
              <span class="font-semibold text-[var(--warning)]">Copy now — plaintext token is shown only once:</span>
              <button
                type="button"
                onclick={() => {
                  if (createdTokenPlain) navigator.clipboard.writeText(createdTokenPlain);
                  copiedToken = true;
                  setTimeout(() => (copiedToken = false), 2000);
                }}
                class="inline-flex items-center gap-1 text-[var(--primary)] font-medium"
              >
                {#if copiedToken}<Check class="w-3.5 h-3.5" /> Copied{:else}<Copy class="w-3.5 h-3.5" /> Copy{/if}
              </button>
            </div>
            <code class="block font-mono text-[11px] break-all text-[var(--foreground)]">{createdTokenPlain}</code>
          </div>
        {/if}

        <div class="divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded">
          {#if apiTokens.length === 0}
            <div class="p-4 text-xs text-[var(--muted-foreground)] text-center">No active API tokens.</div>
          {:else}
            {#each apiTokens as tk}
              <div class="px-3.5 py-2.5 flex items-center justify-between text-xs">
                <div>
                  <span class="font-medium text-[var(--foreground)]">{tk.name}</span>
                  <span class="ml-2 font-mono text-[11px] text-[var(--muted-foreground)]">{tk.token_prefix}…</span>
                </div>
                <button type="button" onclick={() => handleRevokeToken(tk.id)} class="text-[var(--danger)] hover:underline text-[11px]">
                  Revoke
                </button>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {:else if tab === 'backup'}
    <div class="w-full space-y-5">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
          <div class="flex items-center justify-between">
            <h2 class="text-sm font-semibold text-[var(--foreground)] flex items-center gap-2">
              <DatabaseBackup class="w-4 h-4 text-[var(--primary)]" />
              SQLite Online Snapshots (`VACUUM INTO`)
            </h2>
            {#if canWriteSettings}
              <button
                type="button"
                disabled={busy}
                onclick={handleCreateSnapshot}
                class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium disabled:opacity-50"
              >
                Create Snapshot
              </button>
            {/if}
          </div>
          <div class="divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded">
            {#if snapshots.length === 0}
              <div class="p-4 text-xs text-[var(--muted-foreground)] text-center">No snapshots yet.</div>
            {:else}
              {#each snapshots as sn}
                <div class="px-3.5 py-2.5 flex items-center justify-between text-xs font-mono">
                  <div>
                    <div class="text-[var(--foreground)]">{sn.filename}</div>
                    <div class="text-[11px] text-[var(--muted-foreground)] tabular-nums">
                      {(sn.size_bytes / 1024).toFixed(1)} KB · {new Date(sn.created_at).toLocaleString()}
                    </div>
                  </div>
                  <a
                    href={`/api/v1/backup/snapshots/${encodeURIComponent(sn.filename)}`}
                    class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] border border-[var(--border)] text-[var(--primary)] inline-flex items-center gap-1 font-sans text-[11px]"
                  >
                    <Download class="w-3 h-3" /> Download
                  </a>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
          <div class="flex items-center justify-between">
            <h2 class="text-sm font-semibold text-[var(--foreground)]">Settings JSON (SHA-256 Verified)</h2>
            {#if canWriteSettings}
              <button
                type="button"
                onclick={handleExportSettings}
                class="px-3 py-1.5 rounded bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5"
              >
                <Download class="w-3.5 h-3.5" /> Export JSON
              </button>
            {/if}
          </div>
          {#if canWriteSettings}
            <form onsubmit={handleImportSettings} class="space-y-2">
              <textarea
                rows="5"
                placeholder="Paste exported Settings JSON bundle here to validate SHA-256 & restore…"
                bind:value={importJsonText}
                class="w-full p-2.5 rounded bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"
              ></textarea>
              <button
                type="submit"
                disabled={!importJsonText.trim()}
                class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-40"
              >
                <Upload class="w-3.5 h-3.5" /> Validate & Restore Settings
              </button>
            </form>
          {/if}
        </div>
      </div>
    </div>
  {:else if tab === 'audit'}
    <div class="w-full space-y-3">
      <!-- Server-Side Search + MultiSelect + DateRange Toolbar -->
      <div class="flex flex-wrap items-center gap-2">
        <div class="relative">
          <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Search action, actor, resource…"
            bind:value={auditSearch}
            oninput={() => (auditPage = 1)}
            class="pl-8 pr-3 py-1.5 w-60 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
          />
        </div>
        <MultiSelect
          class="w-40"
          searchable={false}
          placeholder="Status"
          bind:values={auditStatuses}
          onchange={() => (auditPage = 1)}
          options={[
            { value: 'success', label: 'Success' },
            { value: 'denied', label: 'Denied' },
            { value: 'failure', label: 'Failure' },
          ]}
        />
        <MultiSelect
          class="w-44"
          placeholder="Resource Type"
          bind:values={auditResources}
          onchange={() => (auditPage = 1)}
          options={[
            { value: 'session', label: 'Session' },
            { value: 'user', label: 'User' },
            { value: 'setting', label: 'Setting' },
            { value: 'container', label: 'Container' },
            { value: 'image', label: 'Image' },
            { value: 'volume', label: 'Volume' },
            { value: 'network', label: 'Network' },
            { value: 'database', label: 'Database' },
            { value: 'redis', label: 'Redis' },
            { value: 'server', label: 'Server' },
          ]}
        />
        <DateRangePicker
          bind:from={auditFrom}
          bind:to={auditTo}
          onchange={() => (auditPage = 1)}
        />
      </div>

      <div class="rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
              <th class="px-4 py-2.5">
                <SortHeader label="Action" column="action" activeColumn={auditSortBy} direction={auditSortDir} onSort={sortAudit} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Resource" column="resource_type" activeColumn={auditSortBy} direction={auditSortDir} onSort={sortAudit} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Actor" column="actor_email" activeColumn={auditSortBy} direction={auditSortDir} onSort={sortAudit} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Status" column="status" activeColumn={auditSortBy} direction={auditSortDir} onSort={sortAudit} />
              </th>
              <th class="px-4 py-2.5 text-right">
                <SortHeader label="Timestamp (UTC)" column="created_at" activeColumn={auditSortBy} direction={auditSortDir} align="right" onSort={sortAudit} />
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
            {#each auditLogs as ev}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2 font-mono text-[var(--foreground)]">{ev.action}</td>
                <td class="px-4 py-2 font-mono text-[var(--muted-foreground)]">{ev.resource_type}</td>
                <td class="px-4 py-2 text-[var(--muted-foreground)]">{ev.actor_email ?? 'system'}</td>
                <td class="px-4 py-2">
                  <span class="px-1.5 py-0.5 rounded text-[11px] {ev.status === 'success' ? 'bg-[var(--success-subtle)] text-[var(--success)]' : 'bg-[var(--danger-subtle)] text-[var(--danger)]'}">
                    {ev.status}
                  </span>
                </td>
                <td class="px-4 py-2 text-right font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                  {ev.created_at.slice(0, 19).replace('T', ' ')}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <PaginationBar
          page={auditMeta.page}
          perPage={auditMeta.per_page}
          total={auditMeta.total}
          totalPages={auditMeta.total_pages}
          onPageChange={(p) => (auditPage = p)}
          onPerPageChange={(pp) => {
            auditPerPage = pp;
            auditPage = 1;
          }}
        />
      </div>
    </div>
  {/if}
  </ExplorerTabs>
</div>
