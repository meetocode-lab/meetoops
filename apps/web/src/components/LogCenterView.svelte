<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type ContainerSummaryDto, type LogLineDto } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import Select from './ui/Select.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import { Pause, Play, Download, Trash2, Search, AlertCircle, ScrollText } from 'lucide-svelte';

  interface Props {
    lang: Language;
    initialContainerId: string | null;
  }

  let { lang, initialContainerId }: Props = $props();

  const MAX_BUFFER_LINES = 5000;

  let containers = $state<ContainerSummaryDto[]>([]);
  let selectedId = $state<string>(untrack(() => initialContainerId ?? ''));
  let lines = $state<LogLineDto[]>([]);
  let searchFilter = $state('');
  let paused = $state(false);
  let follow = $state(true);
  let connected = $state(false);
  let errorMsg = $state<string | null>(null);
  let logContainer = $state<HTMLDivElement | null>(null);
  let activeSocket: WebSocket | null = null;

  const visibleLines = $derived(
    searchFilter.trim()
      ? lines.filter((l) => l.line.toLowerCase().includes(searchFilter.toLowerCase()))
      : lines,
  );

  async function loadContainers() {
    errorMsg = null;
    try {
      containers = await api.listContainers(true);
      if (!selectedId && containers.length > 0) {
        selectedId = containers[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Unable to load container list.';
    }
  }

  function disconnect() {
    if (activeSocket) {
      activeSocket.close();
      activeSocket = null;
    }
    connected = false;
  }

  function connectStream(cid: string) {
    disconnect();
    lines = [];
    if (!cid) return;

    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${proto}//${window.location.host}/ws/v1/docker/containers/${encodeURIComponent(cid)}/logs?tail=250&timestamps=true`;
    const ws = new WebSocket(wsUrl);
    activeSocket = ws;

    ws.onopen = () => {
      connected = true;
      errorMsg = null;
    };

    ws.onmessage = (event) => {
      try {
        const entry = JSON.parse(event.data) as LogLineDto;
        if (lines.length >= MAX_BUFFER_LINES) {
          lines = [...lines.slice(lines.length - MAX_BUFFER_LINES + 1), entry];
        } else {
          lines = [...lines, entry];
        }
        if (follow && logContainer) {
          queueMicrotask(() => {
            if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
          });
        }
      } catch {
        // Ignore malformed frame
      }
    };

    ws.onerror = () => {
      errorMsg = 'Log WebSocket connection error.';
    };

    ws.onclose = () => {
      connected = false;
    };
  }

  function togglePause() {
    paused = !paused;
    if (activeSocket && activeSocket.readyState === WebSocket.OPEN) {
      activeSocket.send(JSON.stringify({ type: paused ? 'pause' : 'resume' }));
    }
  }

  function downloadLogs() {
    const content = visibleLines.map((l) => `[${l.stream}] ${l.line}`).join('\n');
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `meetoops-${selectedId.slice(0, 12) || 'logs'}.log`;
    a.click();
    URL.revokeObjectURL(url);
  }

  $effect(() => {
    loadContainers();
    return () => disconnect();
  });

  $effect(() => {
    const cid = selectedId;
    if (cid) {
      connectStream(cid);
    }
    return () => disconnect();
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight">
        {lang === 'id' ? 'Pusat Log (Real-Time)' : 'Log Center'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Streaming log kontainer langsung melalui WebSocket dengan buffer terbatas (maks 5.000 baris).'
          : 'Real-time container log streaming over WebSocket with bounded 5,000-line ring buffer.'}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <Select
        class="min-w-56"
        bind:value={selectedId}
        placeholder={lang === 'id' ? 'Pilih kontainer…' : 'Select container…'}
        emptyText={lang === 'id' ? 'Tidak ada kontainer' : 'No containers'}
        options={containers.map((c) => ({
          value: c.id,
          label: c.name,
          description: `${c.state} · ${c.short_id}`,
        }))}
      />

      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder={lang === 'id' ? 'Filter log…' : 'Filter logs…'}
          bind:value={searchFilter}
          class="h-[34px] pl-8 pr-3 w-48 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--foreground)]"
        />
      </div>

      <button
        type="button"
        onclick={togglePause}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5 hover:bg-[var(--surface-elevated)] transition-colors"
      >
        {#if paused}<Play class="w-3.5 h-3.5 text-[var(--success)]" /> Resume{:else}<Pause class="w-3.5 h-3.5 text-[var(--warning)]" /> Pause{/if}
      </button>

      <div class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] inline-flex items-center">
        <Checkbox bind:checked={follow} label="Follow" />
      </div>

      <button
        type="button"
        onclick={downloadLogs}
        disabled={visibleLines.length === 0}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5 hover:bg-[var(--surface-elevated)] transition-colors disabled:opacity-40"
      >
        <Download class="w-3.5 h-3.5" /> Download
      </button>

      <button
        type="button"
        onclick={() => (lines = [])}
        class="h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--muted-foreground)] hover:text-[var(--danger)] hover:bg-[var(--surface-elevated)] transition-colors"
        title="Clear"
      >
        <Trash2 class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-hidden">
    <div class="px-4 py-2 bg-[var(--surface-secondary)] border-b border-[var(--border)] flex items-center justify-between text-[11px] text-[var(--muted-foreground)]">
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full {connected ? 'bg-[var(--success)]' : 'bg-[var(--muted-foreground)]'}"></span>
        <span>{connected ? (paused ? 'Paused' : 'Streaming live') : 'Disconnected'}</span>
      </div>
      <span class="font-mono tabular-nums">{visibleLines.length} / {MAX_BUFFER_LINES} lines</span>
    </div>

    <div
      bind:this={logContainer}
      class="h-[62vh] overflow-y-auto p-4 font-mono text-xs leading-relaxed bg-[var(--background)] space-y-0.5"
    >
      {#if visibleLines.length === 0}
        <div class="h-full flex flex-col items-center justify-center text-[var(--muted-foreground)] gap-2">
          <ScrollText class="w-5 h-5" />
          <span>{lang === 'id' ? 'Belum ada baris log.' : 'No log lines received yet.'}</span>
        </div>
      {:else}
        {#each visibleLines as item}
          <div class="whitespace-pre-wrap break-all {item.stream === 'stderr' ? 'text-[var(--danger)]' : 'text-[var(--foreground)]'}">
            {item.line}
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
