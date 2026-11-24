<script lang="ts">
  import {
    api,
    type PaginationMeta,
    type RemoteTokenResponse,
    type ServerSummary,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import { Server, Plus, Trash2, Search, AlertCircle, Copy, Check } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canWrite: boolean;
    canDelete: boolean;
  }

  let { lang, canWrite, canDelete }: Props = $props();

  let servers = $state<ServerSummary[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);

  let search = $state('');
  let statusFilters = $state<string[]>([]);
  let typeFilters = $state<string[]>([]);
  let sortBy = $state('created_at');
  let sortDir = $state<'asc' | 'desc'>('desc');
  let page = $state(1);
  let perPage = $state(10);

  let showAddRemote = $state(false);
  let remoteName = $state('');
  let remoteGroup = $state('Production');
  let remoteToken = $state<RemoteTokenResponse | null>(null);
  let copied = $state(false);

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      const res = await api.queryServers({
        search: search || undefined,
        status: statusFilters.join(',') || undefined,
        connection_type: typeFilters.join(',') || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      servers = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load servers.';
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

  async function handleAddRemote(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    try {
      remoteToken = await api.createRemoteServerToken(remoteName, remoteGroup);
      remoteName = '';
      await load();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to generate agent token.';
    }
  }

  async function handleDelete(s: ServerSummary) {
    const ok = await confirmDialog({
      title: 'Remove Server',
      message: 'Remove this server and revoke its remote agent credentials?',
      targetName: s.name,
      confirmText: 'Remove Server',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteServer(s.id);
      toast.success('Server removed', s.name);
      await load();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to remove server.';
    }
  }

  function copyCmd() {
    if (!remoteToken) return;
    navigator.clipboard.writeText(remoteToken.install_command);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  $effect(() => {
    void search;
    void statusFilters;
    void typeFilters;
    void sortBy;
    void sortDir;
    void page;
    void perPage;
    load();
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <Server class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Manajemen Multi-Server & Agent' : 'Servers & Remote Agents'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        Manage local Docker hosts and outbound TLS remote servers via `meetoops-agent`.
      </p>
    </div>

    {#if canWrite}
      <button
        type="button"
        onclick={() => (showAddRemote = !showAddRemote)}
        class="px-3 py-1.5 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5"
      >
        <Plus class="w-3.5 h-3.5" /> Add Remote Server
      </button>
    {/if}
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if showAddRemote}
    <div class="w-full p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-3">
      <form onsubmit={handleAddRemote} class="grid grid-cols-1 sm:grid-cols-3 gap-3 items-end">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Server Name</span>
          <input type="text" required placeholder="prod-sg-01" bind:value={remoteName} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Group</span>
          <input type="text" bind:value={remoteGroup} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <button type="submit" class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium">
          Generate Outbound TLS Token (15m)
        </button>
      </form>

      {#if remoteToken}
        <div class="p-3 rounded bg-[var(--background)] border border-[var(--border)] space-y-2">
          <div class="flex items-center justify-between text-[11px]">
            <span class="text-[var(--success)] font-medium">Token ready — expires {remoteToken.expires_at}</span>
            <button type="button" onclick={copyCmd} class="inline-flex items-center gap-1 text-[var(--primary)] font-medium">
              {#if copied}<Check class="w-3.5 h-3.5" /> Copied{:else}<Copy class="w-3.5 h-3.5" /> Copy Command{/if}
            </button>
          </div>
          <pre class="text-[11px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-all">{remoteToken.install_command}</pre>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Server-Side Toolbar -->
  <div class="flex flex-wrap items-center gap-2">
    <div class="relative">
      <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        placeholder="Search server name, group, endpoint…"
        bind:value={search}
        oninput={() => (page = 1)}
        class="pl-8 pr-3 py-1.5 w-64 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
      />
    </div>
    <MultiSelect
      class="w-40"
      searchable={false}
      placeholder="Status"
      bind:values={statusFilters}
      onchange={() => (page = 1)}
      options={[
        { value: 'online', label: 'Online' },
        { value: 'offline', label: 'Offline' },
        { value: 'pending', label: 'Pending' },
      ]}
    />
    <MultiSelect
      class="w-44"
      searchable={false}
      placeholder="Connection Type"
      bind:values={typeFilters}
      onchange={() => (page = 1)}
      options={[
        { value: 'local_socket', label: 'Local Socket' },
        { value: 'remote_agent', label: 'Remote Agent' },
      ]}
    />
  </div>

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
    {#if loading && servers.length === 0}
      <TableSkeleton rows={5} columns={6} />
    {:else if servers.length === 0}
      <div class="p-10 text-center space-y-2">
        <Server class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">No servers registered yet.</div>
      </div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-left border-collapse text-xs">
          <thead>
            <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
              <th class="px-4 py-2.5">
                <SortHeader label="Name" column="name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Group" column="group_name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Type" column="connection_type" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
              </th>
              <th class="px-4 py-2.5">
                <SortHeader label="Status" column="status" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
              </th>
              <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Docker / OS</th>
              <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)] text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--border-subtle)]">
            {#each servers as s}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">{s.name}</td>
                <td class="px-4 py-2.5 text-[var(--muted-foreground)]">{s.group_name}</td>
                <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)]">{s.connection_type}</td>
                <td class="px-4 py-2.5">
                  <span class="px-2 py-0.5 rounded-full text-[11px] font-medium {s.status === 'online' ? 'bg-[var(--success-subtle)] text-[var(--success)]' : s.status === 'pending' ? 'bg-[var(--warning-subtle)] text-[var(--warning)]' : 'bg-[var(--danger-subtle)] text-[var(--danger)]'}">
                    {s.status}
                  </span>
                </td>
                <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)]">
                  {s.docker_version ?? '—'} · {s.os_info ?? '—'}
                </td>
                <td class="px-4 py-2.5 text-right">
                  {#if canDelete}
                    <button type="button" onclick={() => handleDelete(s)} class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]" title="Remove Server">
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
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
</div>
