<script lang="ts">
  import { untrack } from 'svelte';
  import {
    api,
    type ColumnInfoDto,
    type DiscoveredServiceDto,
    type QueryResultDto,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import { Database, Table2, Play, Shield, AlertCircle, Loader2, Download } from 'lucide-svelte';

  interface Props {
    lang: Language;
    initialServiceId: string | null;
  }

  let { lang, initialServiceId }: Props = $props();

  let services = $state<DiscoveredServiceDto[]>([]);
  let selectedServiceId = $state<string>(untrack(() => initialServiceId ?? ''));
  let databases = $state<string[]>([]);
  let selectedDb = $state('');
  let tables = $state<string[]>([]);
  let selectedTable = $state('');
  let tab = $state<'rows' | 'schema' | 'sql'>('rows');

  let columns = $state<ColumnInfoDto[]>([]);
  let queryResult = $state<QueryResultDto | null>(null);
  let sqlInput = $state('SELECT 1;');
  let offset = $state(0);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  const dbServices = $derived(
    services.filter(
      (s) =>
        ['postgres', 'mysql', 'mariadb', 'mongodb'].includes(s.service_type) &&
        s.readiness_stage === 'explorer_ready',
    ),
  );

  const activeService = $derived(dbServices.find((s) => s.id === selectedServiceId) ?? null);

  async function loadServices() {
    try {
      services = await api.listServices();
      if (!selectedServiceId && dbServices.length > 0) {
        selectedServiceId = dbServices[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load database services.';
    }
  }

  async function loadDatabases(sid: string) {
    if (!sid) return;
    busy = true;
    errorMsg = null;
    databases = [];
    tables = [];
    selectedDb = '';
    selectedTable = '';
    queryResult = null;
    try {
      databases = await api.dbListDatabases(sid);
      if (databases.length > 0) {
        selectedDb = databases[0];
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to list databases.';
    } finally {
      busy = false;
    }
  }

  async function loadTables(sid: string, db: string) {
    if (!sid || !db) return;
    busy = true;
    errorMsg = null;
    tables = [];
    selectedTable = '';
    try {
      tables = await api.dbListTables(sid, db);
      if (tables.length > 0) {
        selectedTable = tables[0];
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to list tables.';
    } finally {
      busy = false;
    }
  }

  async function loadTableData(sid: string, db: string, tbl: string, pageOffset: number) {
    if (!sid || !db || !tbl) return;
    busy = true;
    errorMsg = null;
    try {
      const [cols, rows] = await Promise.all([
        api.dbTableColumns(sid, db, tbl),
        api.dbBrowseRows(sid, db, tbl, 50, pageOffset),
      ]);
      columns = cols;
      queryResult = rows;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load table data.';
    } finally {
      busy = false;
    }
  }

  async function runSql() {
    if (!selectedServiceId || !selectedDb || !sqlInput.trim()) return;
    busy = true;
    errorMsg = null;
    try {
      queryResult = await api.dbExecuteQuery(selectedServiceId, selectedDb, sqlInput);
      toast.success('Query executed', `${queryResult.row_count} row(s) returned`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Query execution failed.';
      errorMsg = msg;
      toast.error('Query failed', msg);
    } finally {
      busy = false;
    }
  }

  function exportCsv() {
    if (!queryResult) return;
    const esc = (v: unknown) => {
      if (v === null || v === undefined) return '';
      const s = typeof v === 'object' ? JSON.stringify(v) : String(v);
      return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const header = queryResult.columns.map(esc).join(',');
    const lines = queryResult.rows.map((r) => r.map(esc).join(','));
    const blob = new Blob([[header, ...lines].join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${selectedTable || selectedDb || 'query'}-export.csv`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('CSV exported', `${queryResult.row_count} row(s)`);
  }

  function exportJson() {
    if (!queryResult) return;
    const objects = queryResult.rows.map((row) => {
      const obj: Record<string, unknown> = {};
      queryResult!.columns.forEach((col, i) => {
        obj[col] = row[i];
      });
      return obj;
    });
    const blob = new Blob([JSON.stringify(objects, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${selectedTable || selectedDb || 'query'}-export.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('JSON exported', `${queryResult.row_count} row(s)`);
  }

  $effect(() => {
    loadServices();
  });

  $effect(() => {
    if (selectedServiceId) loadDatabases(selectedServiceId);
  });

  $effect(() => {
    if (selectedServiceId && selectedDb) loadTables(selectedServiceId, selectedDb);
  });

  $effect(() => {
    if (selectedServiceId && selectedDb && selectedTable && tab !== 'sql') {
      loadTableData(selectedServiceId, selectedDb, selectedTable, offset);
    }
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <Database class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Penjelajah Database' : 'Database Explorer'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        PostgreSQL · MySQL / MariaDB · MongoDB — Native parameterized drivers with Read-Only guard.
      </p>
    </div>

    <div class="flex items-center gap-2">
      <Select
        class="min-w-56"
        bind:value={selectedServiceId}
        placeholder="Select database connection…"
        emptyText="No verified database connections"
        options={dbServices.map((s) => ({
          value: s.id,
          label: s.name,
          description: `${s.service_type} · ${s.host ?? ''}:${s.port ?? ''}`,
        }))}
      />

      {#if activeService?.read_only_mode}
        <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-[var(--info-subtle)] text-[var(--info)] text-[11px] font-medium">
          <Shield class="w-3 h-3" /> Read-Only
        </span>
      {/if}
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if dbServices.length === 0}
    <div class="w-full p-8 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-center text-xs text-[var(--muted-foreground)]">
      Go to <strong>Service Discovery</strong> to scan or add a PostgreSQL, MySQL/MariaDB, or MongoDB connection and verify its credentials first.
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-4">
      <!-- Left Schema Tree -->
      <div class="lg:col-span-3 p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
        <div class="space-y-1">
          <span class="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)] block">Database</span>
          <Select
            class="w-full"
            bind:value={selectedDb}
            placeholder="Select database…"
            options={databases.map((db) => ({ value: db, label: db }))}
          />
        </div>

        <div class="space-y-1">
          <div class="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            {activeService?.service_type === 'mongodb' ? 'Collections' : 'Tables'} ({tables.length})
          </div>
          <div class="max-h-[55vh] overflow-y-auto space-y-0.5">
            {#each tables as tbl}
              <button
                type="button"
                onclick={() => {
                  selectedTable = tbl;
                  offset = 0;
                  if (tab === 'sql') tab = 'rows';
                }}
                class="w-full text-left px-2.5 py-1.5 rounded text-xs font-mono flex items-center gap-2 transition-colors {selectedTable === tbl ? 'bg-[var(--primary-subtle)] text-[var(--primary)] font-medium' : 'text-[var(--muted-foreground)] hover:bg-[var(--surface-elevated)] hover:text-[var(--foreground)]'}"
              >
                <Table2 class="w-3.5 h-3.5 shrink-0" />
                <span class="truncate">{tbl}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

      <!-- Right Data / Schema / SQL Panel -->
      <div class="lg:col-span-9 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-hidden flex flex-col">
        <div class="px-4 pt-2 border-b border-[var(--border)] flex items-center justify-between">
          <div class="flex gap-2">
            <button
              type="button"
              onclick={() => (tab = 'rows')}
              class="px-3 py-2 text-xs font-medium border-b-2 {tab === 'rows' ? 'border-[var(--primary)] text-[var(--primary)]' : 'border-transparent text-[var(--muted-foreground)]'}"
            >
              Records
            </button>
            <button
              type="button"
              onclick={() => (tab = 'schema')}
              class="px-3 py-2 text-xs font-medium border-b-2 {tab === 'schema' ? 'border-[var(--primary)] text-[var(--primary)]' : 'border-transparent text-[var(--muted-foreground)]'}"
            >
              Columns / Schema
            </button>
            {#if activeService?.service_type !== 'mongodb'}
              <button
                type="button"
                onclick={() => (tab = 'sql')}
                class="px-3 py-2 text-xs font-medium border-b-2 {tab === 'sql' ? 'border-[var(--primary)] text-[var(--primary)]' : 'border-transparent text-[var(--muted-foreground)]'}"
              >
                SQL Editor
              </button>
            {/if}
          </div>

          <div class="flex items-center gap-2 pb-1.5">
            {#if queryResult && queryResult.rows.length > 0 && tab !== 'schema'}
              <button
                type="button"
                onclick={exportCsv}
                class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] font-medium text-[var(--foreground)] inline-flex items-center gap-1"
              >
                <Download class="w-3 h-3" /> CSV
              </button>
              <button
                type="button"
                onclick={exportJson}
                class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] font-medium text-[var(--foreground)] inline-flex items-center gap-1"
              >
                <Download class="w-3 h-3" /> JSON
              </button>
            {/if}
            {#if busy}
              <Loader2 class="w-3.5 h-3.5 animate-spin text-[var(--primary)]" />
            {/if}
          </div>
        </div>

        {#if tab === 'sql'}
          <div class="p-4 border-b border-[var(--border)] space-y-2">
            <textarea
              rows="4"
              bind:value={sqlInput}
              class="w-full p-3 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs text-[var(--foreground)] focus:outline-none focus:border-[var(--ring)]"
            ></textarea>
            <div class="flex justify-end">
              <button
                type="button"
                disabled={busy}
                onclick={runSql}
                class="px-3.5 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
              >
                <Play class="w-3.5 h-3.5" /> Execute Query
              </button>
            </div>
          </div>
        {/if}

        {#if tab === 'schema'}
          <div class="overflow-x-auto">
            <table class="w-full text-left border-collapse text-xs">
              <thead>
                <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase text-[var(--muted-foreground)]">
                  <th class="px-4 py-2">Column</th>
                  <th class="px-4 py-2">Type</th>
                  <th class="px-4 py-2">Nullable</th>
                  <th class="px-4 py-2">Default</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-[var(--border-subtle)] font-mono">
                {#each columns as col}
                  <tr>
                    <td class="px-4 py-2 text-[var(--foreground)]">{col.name}</td>
                    <td class="px-4 py-2 text-[var(--primary)]">{col.data_type}</td>
                    <td class="px-4 py-2 text-[var(--muted-foreground)]">{col.is_nullable ? 'YES' : 'NO'}</td>
                    <td class="px-4 py-2 text-[var(--muted-foreground)]">{col.default_value ?? '—'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else if queryResult}
          <div class="overflow-x-auto flex-1">
            <table class="w-full text-left border-collapse text-xs">
              <thead>
                <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] font-mono text-[var(--muted-foreground)]">
                  {#each queryResult.columns as col}
                    <th class="px-3 py-2 font-medium whitespace-nowrap">{col}</th>
                  {/each}
                </tr>
              </thead>
              <tbody class="divide-y divide-[var(--border-subtle)] font-mono">
                {#each queryResult.rows as row}
                  <tr class="hover:bg-[var(--surface-elevated)]">
                    {#each row as cell}
                      <td class="px-3 py-2 max-w-[320px] truncate text-[var(--foreground)]">
                        {cell === null ? 'NULL' : typeof cell === 'object' ? JSON.stringify(cell) : String(cell)}
                      </td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          {#if tab === 'rows'}
            <div class="px-4 py-2 border-t border-[var(--border)] flex items-center justify-between text-xs text-[var(--muted-foreground)]">
              <span class="tabular-nums">Showing {queryResult.row_count} rows (offset {offset})</span>
              <div class="flex gap-2">
                <button
                  type="button"
                  disabled={offset === 0}
                  onclick={() => (offset = Math.max(0, offset - 50))}
                  class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] border border-[var(--border)] disabled:opacity-40"
                >
                  Prev
                </button>
                <button
                  type="button"
                  disabled={queryResult.row_count < 50}
                  onclick={() => (offset += 50)}
                  class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] border border-[var(--border)] disabled:opacity-40"
                >
                  Next
                </button>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>
