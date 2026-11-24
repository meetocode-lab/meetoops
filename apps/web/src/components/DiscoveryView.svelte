<script lang="ts">
  import { api, type DiscoveredServiceDto, type PaginationMeta } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import { Radar, Plus, KeyRound, Trash2, CheckCircle2, AlertCircle, ExternalLink, X, Loader2, Search } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canManageCredentials: boolean;
    onOpenDbExplorer: (serviceId: string) => void;
    onOpenRedisExplorer: (serviceId: string) => void;
  }

  let { lang, canManageCredentials, onOpenDbExplorer, onOpenRedisExplorer }: Props = $props();

  let services = $state<DiscoveredServiceDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let scanning = $state(false);
  let errorMsg = $state<string | null>(null);
  let statusMsg = $state<string | null>(null);

  // Server-side filters & pagination
  let searchQuery = $state('');
  let typeFilters = $state<string[]>([]);
  let stageFilters = $state<string[]>([]);
  let sortBy = $state('created_at');
  let sortDir = $state<'asc' | 'desc'>('desc');
  let page = $state(1);
  let perPage = $state(10);

  // Manual connection form
  let showAddForm = $state(false);
  let manualName = $state('');
  let manualType = $state('postgres');
  let manualHost = $state('127.0.0.1');
  let manualPort = $state(5432);
  let manualReadOnly = $state(true);

  // Credentials panel
  let editingService = $state<DiscoveredServiceDto | null>(null);
  let credHost = $state('127.0.0.1');
  let credPort = $state(5432);
  let credUser = $state('');
  let credPass = $state('');
  let credDb = $state('');
  let credReadOnly = $state(true);
  let savingCreds = $state(false);

  function defaultPortFor(t: string): number {
    if (t === 'postgres') return 5432;
    if (t === 'mysql' || t === 'mariadb') return 3306;
    if (t === 'mongodb') return 27017;
    if (t === 'redis') return 6379;
    return 5432;
  }

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      const res = await api.queryServices({
        search: searchQuery || undefined,
        service_type: typeFilters.join(',') || undefined,
        readiness_stage: stageFilters.join(',') || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      services = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load services.';
    } finally {
      loading = false;
    }
  }

  function handleSort(col: string) {
    if (sortBy === col) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    else {
      sortBy = col;
      sortDir = 'asc';
    }
    page = 1;
  }

  $effect(() => {
    void searchQuery;
    void typeFilters;
    void stageFilters;
    void sortBy;
    void sortDir;
    void page;
    void perPage;
    load();
  });

  async function handleScan() {
    scanning = true;
    errorMsg = null;
    statusMsg = null;
    try {
      services = await api.scanServices();
      statusMsg = lang === 'id' ? 'Pemindaian kontainer selesai.' : 'Container scan completed.';
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Docker scan failed.';
    } finally {
      scanning = false;
    }
  }

  async function handleAddManual(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    try {
      const created = await api.createManualService({
        name: manualName,
        service_type: manualType,
        host: manualHost,
        port: Number(manualPort),
        read_only_mode: manualReadOnly,
      });
      manualName = '';
      showAddForm = false;
      await load();
      openCredentials(created);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to add service.';
    }
  }

  function openCredentials(svc: DiscoveredServiceDto) {
    editingService = svc;
    credHost = svc.host ?? '127.0.0.1';
    credPort = svc.port ?? defaultPortFor(svc.service_type);
    credUser = svc.service_type === 'postgres' ? 'postgres' : svc.service_type === 'redis' ? '' : 'root';
    credPass = '';
    credDb = '';
    credReadOnly = svc.read_only_mode;
  }

  async function handleSaveCredentials(e: SubmitEvent) {
    e.preventDefault();
    if (!editingService) return;
    savingCreds = true;
    errorMsg = null;
    statusMsg = null;
    try {
      const res = await api.saveServiceCredentials(editingService.id, {
        host: credHost,
        port: Number(credPort),
        username: credUser,
        password: credPass,
        database: credDb || undefined,
        read_only_mode: credReadOnly,
      });
      if (res.auth_verified) {
        statusMsg = `Verified & Explorer Ready: ${res.server_banner ?? 'OK'}`;
        editingService = null;
      } else {
        errorMsg = `Credentials encrypted & saved, but authentication failed: ${res.verify_error ?? 'Unknown error'}`;
      }
      await load();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to save credentials.';
    } finally {
      savingCreds = false;
    }
  }

  async function handleDelete(svc: DiscoveredServiceDto) {
    const ok = await confirmDialog({
      title: 'Remove Service Connection',
      message: 'Remove this service connection and its encrypted credentials from the vault?',
      targetName: `${svc.name} (${svc.service_type})`,
      confirmText: 'Remove Connection',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteService(svc.id);
      toast.success('Service removed', svc.name);
      if (editingService?.id === svc.id) editingService = null;
      await load();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to delete service.';
    }
  }

  function stageBadge(stage: string): string {
    if (stage === 'explorer_ready' || stage === 'auth_verified')
      return 'bg-[var(--success-subtle)] text-[var(--success)]';
    if (stage === 'credentials_available' || stage === 'connection_discovered')
      return 'bg-[var(--info-subtle)] text-[var(--info)]';
    return 'bg-[var(--warning-subtle)] text-[var(--warning)]';
  }
</script>

<div class="w-full space-y-5">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <Radar class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Deteksi Layanan & Koneksi' : 'Smart Service Discovery'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Deteksi otomatis kontainer database/Redis atau tambahkan koneksi manual (kredensial dienkripsi AES-256-GCM).'
          : 'Auto-detect running containers or register external databases/Redis. Credentials are encrypted with AES-256-GCM.'}
      </p>
    </div>

    <div class="flex items-center gap-2">
      <button
        type="button"
        disabled={scanning}
        onclick={handleScan}
        class="px-3 py-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-xs inline-flex items-center gap-1.5 disabled:opacity-50"
      >
        {#if scanning}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Radar class="w-3.5 h-3.5 text-[var(--primary)]" />{/if}
        {lang === 'id' ? 'Pindai Docker' : 'Scan Containers'}
      </button>
      {#if canManageCredentials}
        <button
          type="button"
          onclick={() => (showAddForm = !showAddForm)}
          class="px-3 py-1.5 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Plus class="w-3.5 h-3.5" />
          {lang === 'id' ? 'Tambah Koneksi' : 'Add Connection'}
        </button>
      {/if}
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}
  {#if statusMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--success-subtle)] border border-[var(--success)]/40 text-[var(--success)] text-xs">
      <CheckCircle2 class="w-4 h-4 shrink-0" />
      <span>{statusMsg}</span>
    </div>
  {/if}

  {#if showAddForm}
    <form
      onsubmit={handleAddManual}
      class="w-full p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] grid grid-cols-1 md:grid-cols-6 gap-3 items-end"
    >
      <label class="space-y-1">
        <span class="text-[11px] text-[var(--muted-foreground)]">Connection Name</span>
        <input type="text" required placeholder="prod-postgres" bind:value={manualName} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs" />
      </label>
      <div class="space-y-1">
        <span class="text-[11px] text-[var(--muted-foreground)] block">Engine</span>
        <Select
          class="w-full"
          searchable={false}
          bind:value={manualType}
          onchange={(v) => (manualPort = defaultPortFor(v))}
          options={[
            { value: 'postgres', label: 'PostgreSQL' },
            { value: 'mysql', label: 'MySQL' },
            { value: 'mariadb', label: 'MariaDB' },
            { value: 'mongodb', label: 'MongoDB' },
            { value: 'redis', label: 'Redis' },
          ]}
        />
      </div>
      <label class="space-y-1">
        <span class="text-[11px] text-[var(--muted-foreground)]">Host</span>
        <input type="text" required bind:value={manualHost} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono" />
      </label>
      <label class="space-y-1">
        <span class="text-[11px] text-[var(--muted-foreground)]">Port</span>
        <input type="number" required bind:value={manualPort} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono tabular-nums" />
      </label>
      <div class="pb-1.5">
        <Checkbox bind:checked={manualReadOnly} label="Read-Only Default" />
      </div>
      <button type="submit" class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium">
        Create & Configure
      </button>
    </form>
  {/if}

  <!-- Server-Side Search & MultiSelect Toolbar -->
  <div class="flex flex-wrap items-center gap-2">
    <div class="relative">
      <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        placeholder="Search service, host, type…"
        bind:value={searchQuery}
        oninput={() => (page = 1)}
        class="pl-8 pr-3 py-1.5 w-60 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
      />
    </div>
    <MultiSelect
      class="w-44"
      placeholder="Engine / Type"
      bind:values={typeFilters}
      onchange={() => (page = 1)}
      options={[
        { value: 'postgres', label: 'PostgreSQL' },
        { value: 'mysql', label: 'MySQL' },
        { value: 'mariadb', label: 'MariaDB' },
        { value: 'mongodb', label: 'MongoDB' },
        { value: 'redis', label: 'Redis' },
        { value: 'rabbitmq', label: 'RabbitMQ' },
        { value: 'elasticsearch', label: 'Elasticsearch' },
        { value: 'nginx', label: 'Nginx' },
      ]}
    />
    <MultiSelect
      class="w-48"
      searchable={false}
      placeholder="Readiness Stage"
      bind:values={stageFilters}
      onchange={() => (page = 1)}
      options={[
        { value: 'detected', label: 'Detected' },
        { value: 'connection_discovered', label: 'Connection Discovered' },
        { value: 'credentials_available', label: 'Credentials Available' },
        { value: 'explorer_ready', label: 'Explorer Ready' },
      ]}
    />
  </div>

  <!-- Services Table -->
  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
    {#if loading && services.length === 0}
      <TableSkeleton rows={5} columns={6} />
    {:else if services.length === 0}
      <div class="p-10 text-center space-y-2">
        <Radar class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">
          No services discovered or registered yet. Scan Docker containers or add a database/Redis connection manually.
        </div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
            <th class="px-4 py-2.5">
              <SortHeader label="Service" column="name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Type" column="service_type" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Endpoint</th>
            <th class="px-4 py-2.5">
              <SortHeader label="Confidence" column="confidence" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Readiness Stage" column="readiness_stage" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
            </th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)] text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
          {#each services as svc}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">
                {svc.name}
                {#if svc.read_only_mode}
                  <span class="ml-1.5 px-1.5 py-0.5 rounded bg-[var(--surface-secondary)] text-[10px] text-[var(--muted-foreground)]">RO</span>
                {/if}
              </td>
              <td class="px-4 py-2.5 font-mono uppercase text-[11px] text-[var(--primary)]">{svc.service_type}</td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums">
                {svc.host ?? '—'}:{svc.port ?? '—'}
              </td>
              <td class="px-4 py-2.5 capitalize text-[var(--muted-foreground)]">{svc.confidence}</td>
              <td class="px-4 py-2.5">
                <span class="px-2 py-0.5 rounded-full text-[11px] font-mono {stageBadge(svc.readiness_stage)}">
                  {svc.readiness_stage}
                </span>
              </td>
              <td class="px-4 py-2.5 text-right whitespace-nowrap">
                <div class="inline-flex items-center gap-1.5">
                  {#if svc.readiness_stage === 'explorer_ready'}
                    {#if svc.service_type === 'redis'}
                      <button
                        type="button"
                        onclick={() => onOpenRedisExplorer(svc.id)}
                        class="px-2.5 py-1 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-[11px] font-medium inline-flex items-center gap-1"
                      >
                        Explorer <ExternalLink class="w-3 h-3" />
                      </button>
                    {:else if ['postgres', 'mysql', 'mariadb', 'mongodb'].includes(svc.service_type)}
                      <button
                        type="button"
                        onclick={() => onOpenDbExplorer(svc.id)}
                        class="px-2.5 py-1 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-[11px] font-medium inline-flex items-center gap-1"
                      >
                        Explorer <ExternalLink class="w-3 h-3" />
                      </button>
                    {/if}
                  {/if}
                  {#if canManageCredentials}
                    <button
                      type="button"
                      onclick={() => openCredentials(svc)}
                      class="px-2 py-1 rounded bg-[var(--surface-secondary)] border border-[var(--border)] text-[11px] inline-flex items-center gap-1"
                    >
                      <KeyRound class="w-3 h-3" /> Credentials
                    </button>
                    <button
                      type="button"
                      onclick={() => handleDelete(svc)}
                      class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                      title="Delete"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <PaginationBar
      page={meta.page}
      perPage={meta.per_page}
      total={meta.total}
      totalPages={meta.total_pages}
      onPageChange={(p) => (page = p)}
      onPerPageChange={(pp) => {
        perPage = pp;
        page = 1;
      }}
    />
  </div>

  <!-- Encrypted Credentials Form Panel -->
  {#if editingService}
    <form
      onsubmit={handleSaveCredentials}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-4"
    >
      <div class="flex items-center justify-between border-b border-[var(--border)] pb-3">
        <div>
          <h2 class="text-sm font-semibold text-[var(--foreground)]">
            Configure Encrypted Credentials — {editingService.name} ({editingService.service_type})
          </h2>
          <p class="text-[11px] text-[var(--muted-foreground)]">
            Encrypted with AES-256-GCM before storage and immediately verified against the target engine.
          </p>
        </div>
        <button type="button" onclick={() => (editingService = null)} class="p-1 text-[var(--muted-foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <label class="space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">Host</span>
          <input type="text" required bind:value={credHost} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono" />
        </label>
        <label class="space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">Port</span>
          <input type="number" required bind:value={credPort} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono tabular-nums" />
        </label>
        <label class="space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">Default Database (optional)</span>
          <input type="text" bind:value={credDb} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono" />
        </label>
        <label class="space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">Username</span>
          <input type="text" bind:value={credUser} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono" />
        </label>
        <label class="space-y-1">
          <span class="text-xs text-[var(--muted-foreground)]">Password</span>
          <input type="password" bind:value={credPass} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono" />
        </label>
        <div class="pt-5">
          <Checkbox bind:checked={credReadOnly} label="Enforce Read-Only Mode (Recommended)" />
        </div>
      </div>

      <div class="flex justify-end gap-2">
        <button type="button" onclick={() => (editingService = null)} class="px-3 py-1.5 rounded bg-[var(--surface-elevated)] border border-[var(--border)] text-xs">
          Cancel
        </button>
        <button type="submit" disabled={savingCreds} class="px-4 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50">
          {#if savingCreds}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
          Save & Verify Connection
        </button>
      </div>
    </form>
  {/if}
</div>
