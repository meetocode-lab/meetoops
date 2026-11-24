<script lang="ts">
  import { untrack } from 'svelte';
  import {
    api,
    type DiscoveredServiceDto,
    type RedisKeyDetailDto,
    type RedisKeyItemDto,
    type RedisOverviewDto,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import { KeyRound, Search, Trash2, Terminal, Save, AlertCircle, RefreshCw, Plus, Pencil } from 'lucide-svelte';

  interface Props {
    lang: Language;
    initialServiceId: string | null;
    canWrite: boolean;
  }

  let { lang, initialServiceId, canWrite }: Props = $props();

  let services = $state<DiscoveredServiceDto[]>([]);
  let selectedServiceId = $state<string>(untrack(() => initialServiceId ?? ''));
  let overview = $state<RedisOverviewDto | null>(null);
  let keys = $state<RedisKeyItemDto[]>([]);
  let nextCursor = $state(0);
  let pattern = $state('*');
  let selectedKey = $state<RedisKeyDetailDto | null>(null);
  let editValue = $state('');
  let editTtl = $state('');
  let renameTarget = $state('');
  let mutField = $state('');
  let mutValue = $state('');
  let mutScore = $state('0');
  let cmdInput = $state('');
  let cmdOutput = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);

  const redisServices = $derived(
    services.filter((s) => s.service_type === 'redis' && s.readiness_stage === 'explorer_ready'),
  );

  async function loadServices() {
    try {
      services = await api.listServices();
      if (!selectedServiceId && redisServices.length > 0) {
        selectedServiceId = redisServices[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load Redis services.';
    }
  }

  async function loadOverviewAndScan(sid: string, cursor = 0, append = false) {
    if (!sid) return;
    errorMsg = null;
    try {
      const [ov, page] = await Promise.all([
        api.redisOverview(sid),
        api.redisScanKeys(sid, cursor, pattern, 50),
      ]);
      overview = ov;
      nextCursor = page.next_cursor;
      keys = append ? [...keys, ...page.keys] : page.keys;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to query Redis.';
    }
  }

  async function openKey(k: string) {
    if (!selectedServiceId) return;
    errorMsg = null;
    try {
      const detail = await api.redisGetKey(selectedServiceId, k);
      selectedKey = detail;
      editValue =
        typeof detail.value === 'string' ? detail.value : JSON.stringify(detail.value, null, 2);
      editTtl = detail.ttl > 0 ? String(detail.ttl) : '';
      renameTarget = detail.key;
      mutField = '';
      mutValue = '';
      mutScore = '0';
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load key.';
    }
  }

  async function saveKey() {
    if (!selectedServiceId || !selectedKey) return;
    errorMsg = null;
    try {
      await api.redisSetKey(selectedServiceId, {
        key: selectedKey.key,
        value: editValue,
        ttl_secs: editTtl ? Number(editTtl) : undefined,
      });
      toast.success('Key updated', selectedKey.key);
      await openKey(selectedKey.key);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to update key.';
      errorMsg = msg;
      toast.error('Update failed', msg);
    }
  }

  async function handleRenameKey() {
    if (!selectedServiceId || !selectedKey || !renameTarget.trim() || renameTarget.trim() === selectedKey.key) {
      return;
    }
    errorMsg = null;
    try {
      const next = renameTarget.trim();
      await api.redisRenameKey(selectedServiceId, selectedKey.key, next);
      toast.success('Key renamed', `${selectedKey.key} → ${next}`);
      await loadOverviewAndScan(selectedServiceId, 0, false);
      await openKey(next);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to rename key.';
      errorMsg = msg;
      toast.error('Rename failed', msg);
    }
  }

  async function handleMutateCollection(e: SubmitEvent) {
    e.preventDefault();
    if (!selectedServiceId || !selectedKey || !mutValue.trim()) return;
    errorMsg = null;
    try {
      await api.redisMutateKey(selectedServiceId, {
        key: selectedKey.key,
        key_type: selectedKey.key_type,
        field: selectedKey.key_type === 'hash' ? mutField.trim() : undefined,
        value: mutValue,
        score: selectedKey.key_type === 'zset' ? Number(mutScore) || 0 : undefined,
      });
      toast.success('Entry added', `${selectedKey.key} (${selectedKey.key_type})`);
      mutField = '';
      mutValue = '';
      await openKey(selectedKey.key);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to mutate collection key.';
      errorMsg = msg;
      toast.error('Mutation failed', msg);
    }
  }

  async function removeKey(k: string) {
    if (!selectedServiceId) return;
    const ok = await confirmDialog({
      title: 'Delete Redis Key',
      message: 'This permanently deletes the key from Redis.',
      targetName: k,
      confirmText: 'Delete Key',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.redisDeleteKey(selectedServiceId, k);
      toast.success('Key deleted', k);
      if (selectedKey?.key === k) selectedKey = null;
      await loadOverviewAndScan(selectedServiceId, 0, false);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to delete key.';
      errorMsg = msg;
      toast.error('Delete failed', msg);
    }
  }

  async function runCommand(e: SubmitEvent) {
    e.preventDefault();
    if (!selectedServiceId || !cmdInput.trim()) return;
    errorMsg = null;
    try {
      const res = await api.redisCommand(selectedServiceId, cmdInput, false);
      cmdOutput = res.output;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Command failed.';
    }
  }

  $effect(() => {
    loadServices();
  });

  $effect(() => {
    if (selectedServiceId) loadOverviewAndScan(selectedServiceId, 0, false);
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <KeyRound class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Penjelajah Redis' : 'Redis Explorer'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        Cursor-based SCAN browser (never KEYS *), live INFO telemetry, and guarded command console.
      </p>
    </div>

    <div class="flex items-center gap-2">
      <Select
        class="min-w-56"
        bind:value={selectedServiceId}
        placeholder="Select Redis connection…"
        emptyText="No verified Redis connections"
        options={redisServices.map((s) => ({
          value: s.id,
          label: s.name,
          description: `${s.host ?? ''}:${s.port ?? ''}`,
        }))}
      />
      <button
        type="button"
        onclick={() => loadOverviewAndScan(selectedServiceId, 0, false)}
        class="px-3 py-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs inline-flex items-center gap-1.5"
      >
        <RefreshCw class="w-3.5 h-3.5" /> Refresh
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if redisServices.length === 0}
    <div class="w-full p-8 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-center text-xs text-[var(--muted-foreground)]">
      Go to <strong>Service Discovery</strong> to scan or add a Redis connection and verify its credentials first.
    </div>
  {:else}
    {#if overview}
      <div class="grid grid-cols-2 lg:grid-cols-5 gap-3">
        <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
          <div class="text-[11px] uppercase text-[var(--muted-foreground)]">Memory</div>
          <div class="text-lg font-semibold tabular-nums mt-1">{overview.used_memory_human ?? '—'}</div>
        </div>
        <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
          <div class="text-[11px] uppercase text-[var(--muted-foreground)]">Total Keys</div>
          <div class="text-lg font-semibold tabular-nums mt-1">{overview.total_keys}</div>
        </div>
        <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
          <div class="text-[11px] uppercase text-[var(--muted-foreground)]">Clients</div>
          <div class="text-lg font-semibold tabular-nums mt-1">{overview.connected_clients ?? '—'}</div>
        </div>
        <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
          <div class="text-[11px] uppercase text-[var(--muted-foreground)]">Ops / sec</div>
          <div class="text-lg font-semibold tabular-nums mt-1">{overview.ops_per_sec ?? '—'}</div>
        </div>
        <div class="p-3.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
          <div class="text-[11px] uppercase text-[var(--muted-foreground)]">Hits / Misses</div>
          <div class="text-sm font-semibold font-mono tabular-nums mt-1.5">
            {overview.keyspace_hits ?? '—'} / {overview.keyspace_misses ?? '—'}
          </div>
        </div>
      </div>
    {/if}

    <div class="grid grid-cols-1 lg:grid-cols-12 gap-4">
      <!-- SCAN Key Browser -->
      <div class="lg:col-span-5 p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
        <form
          onsubmit={(e) => {
            e.preventDefault();
            loadOverviewAndScan(selectedServiceId, 0, false);
          }}
          class="flex gap-2"
        >
          <div class="relative flex-1">
            <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              bind:value={pattern}
              placeholder="session:*"
              class="w-full pl-8 pr-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
            />
          </div>
          <button type="submit" class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium">
            SCAN
          </button>
        </form>

        <div class="max-h-[45vh] overflow-y-auto divide-y divide-[var(--border-subtle)] border border-[var(--border)] rounded">
          {#each keys as item}
            <div class="px-3 py-2 flex items-center justify-between gap-2 hover:bg-[var(--surface-elevated)]">
              <button type="button" onclick={() => openKey(item.key)} class="text-left flex-1 truncate font-mono text-xs text-[var(--foreground)]">
                {item.key}
              </button>
              <span class="px-1.5 py-0.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[10px]">
                {item.key_type}
              </span>
              <span class="font-mono text-[10px] text-[var(--muted-foreground)] tabular-nums">
                {item.ttl === -1 ? '∞' : `${item.ttl}s`}
              </span>
              {#if canWrite}
                <button type="button" onclick={() => removeKey(item.key)} class="text-[var(--danger)] p-1">
                  <Trash2 class="w-3 h-3" />
                </button>
              {/if}
            </div>
          {/each}
        </div>

        {#if nextCursor !== 0}
          <button
            type="button"
            onclick={() => loadOverviewAndScan(selectedServiceId, nextCursor, true)}
            class="w-full py-1.5 rounded bg-[var(--surface-secondary)] border border-[var(--border)] text-xs"
          >
            Load More (Cursor {nextCursor})
          </button>
        {/if}
      </div>

      <!-- Key Inspector + Guarded Console -->
      <div class="lg:col-span-7 space-y-4">
        <div class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-3">
          <div class="text-xs font-semibold text-[var(--foreground)]">
            {selectedKey ? `${selectedKey.key} (${selectedKey.key_type})` : 'Select a key to inspect'}
          </div>
          {#if selectedKey}
            {#if canWrite}
              <div class="flex items-center gap-2">
                <input
                  type="text"
                  bind:value={renameTarget}
                  placeholder="New key name"
                  class="flex-1 px-2.5 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
                />
                <button
                  type="button"
                  onclick={handleRenameKey}
                  disabled={!renameTarget.trim() || renameTarget.trim() === selectedKey.key}
                  class="px-2.5 py-1.5 rounded bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-xs font-medium inline-flex items-center gap-1 disabled:opacity-40"
                >
                  <Pencil class="w-3 h-3" /> Rename
                </button>
              </div>
            {/if}
            <textarea
              rows="6"
              bind:value={editValue}
              readonly={!canWrite || selectedKey.key_type !== 'string'}
              class="w-full p-3 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
            ></textarea>
            {#if canWrite && selectedKey.key_type === 'string'}
              <div class="flex items-center justify-between gap-2">
                <input
                  type="number"
                  placeholder="TTL (seconds, optional)"
                  bind:value={editTtl}
                  class="px-3 py-1.5 w-48 rounded bg-[var(--background)] border border-[var(--border)] text-xs font-mono"
                />
                <button type="button" onclick={saveKey} class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5">
                  <Save class="w-3.5 h-3.5" /> Save String
                </button>
              </div>
            {:else if canWrite && ['hash', 'list', 'set', 'zset'].includes(selectedKey.key_type)}
              <form onsubmit={handleMutateCollection} class="flex flex-wrap items-center gap-2 pt-1">
                {#if selectedKey.key_type === 'hash'}
                  <input
                    type="text"
                    required
                    placeholder="Field name"
                    bind:value={mutField}
                    class="px-2.5 py-1.5 w-36 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
                  />
                {/if}
                {#if selectedKey.key_type === 'zset'}
                  <input
                    type="number"
                    step="any"
                    placeholder="Score"
                    bind:value={mutScore}
                    class="px-2.5 py-1.5 w-24 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
                  />
                {/if}
                <input
                  type="text"
                  required
                  placeholder={selectedKey.key_type === 'hash' ? 'Field value' : 'New member / item'}
                  bind:value={mutValue}
                  class="flex-1 min-w-36 px-2.5 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
                />
                <button
                  type="submit"
                  class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1"
                >
                  <Plus class="w-3.5 h-3.5" /> Add to {selectedKey.key_type}
                </button>
              </form>
            {/if}
          {/if}
        </div>

        <form onsubmit={runCommand} class="p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-2.5">
          <div class="text-xs font-semibold text-[var(--foreground)] flex items-center gap-1.5">
            <Terminal class="w-3.5 h-3.5 text-[var(--primary)]" />
            Guarded Redis Command Console
          </div>
          <div class="flex gap-2">
            <input
              type="text"
              placeholder="PING or GET mykey"
              bind:value={cmdInput}
              class="flex-1 px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs"
            />
            <button type="submit" disabled={!canWrite} class="px-3.5 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium disabled:opacity-40">
              Run
            </button>
          </div>
          {#if cmdOutput !== null}
            <pre class="p-2.5 rounded bg-[var(--background)] border border-[var(--border)] font-mono text-xs overflow-x-auto">{cmdOutput}</pre>
          {/if}
        </form>
      </div>
    </div>
  {/if}
</div>
