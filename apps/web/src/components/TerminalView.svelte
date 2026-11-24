<script lang="ts">
  import { untrack } from 'svelte';
  import { Terminal as XTerm } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';
  import { api, type ContainerSummaryDto } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import Select from './ui/Select.svelte';
  import { Terminal, Plug, Unplug, Trash2, AlertCircle } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canExec: boolean;
    initialContainerId?: string | null;
  }

  let { lang, canExec, initialContainerId = null }: Props = $props();

  let containers = $state<ContainerSummaryDto[]>([]);
  let selectedId = $state(untrack(() => initialContainerId ?? ''));
  let shell = $state('/bin/sh');
  let connected = $state(false);
  let errorMsg = $state<string | null>(null);
  let termEl = $state<HTMLDivElement | null>(null);

  let term: XTerm | null = null;
  let fitAddon: FitAddon | null = null;
  let socket: WebSocket | null = null;

  async function loadContainers() {
    errorMsg = null;
    try {
      const res = await api.queryContainers({ all: false, state: 'running', per_page: 100 });
      containers = res.items;
      if (!selectedId && containers.length > 0) {
        selectedId = containers[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Unable to load running containers.';
    }
  }

  function disconnect() {
    if (socket) {
      socket.close();
      socket = null;
    }
    connected = false;
  }

  function connect() {
    if (!selectedId || !canExec || !term) return;
    disconnect();
    errorMsg = null;
    term.clear();
    term.writeln(`\x1b[36mConnecting to container ${selectedId.slice(0, 12)} (${shell})…\x1b[0m`);

    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${proto}//${window.location.host}/ws/v1/docker/containers/${encodeURIComponent(selectedId)}/exec?shell=${encodeURIComponent(shell)}`;
    const ws = new WebSocket(wsUrl);
    socket = ws;

    ws.onopen = () => {
      connected = true;
      fitAddon?.fit();
      if (term) {
        ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
      }
      term?.focus();
    };

    ws.onmessage = (ev) => {
      term?.write(typeof ev.data === 'string' ? ev.data : '');
    };

    ws.onerror = () => {
      errorMsg = 'Terminal WebSocket error (check Docker connection & permissions).';
    };

    ws.onclose = () => {
      connected = false;
      term?.writeln('\r\n\x1b[33m[Session closed]\x1b[0m');
    };
  }

  $effect(() => {
    loadContainers();
  });

  $effect(() => {
    if (!termEl) return;
    const xterm = new XTerm({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: "'JetBrains Mono', ui-monospace, monospace",
      theme: {
        background: '#071426',
        foreground: '#f8fafc',
        cursor: '#3b82f6',
        selectionBackground: 'rgba(59, 130, 246, 0.35)',
        green: '#4ade80',
        red: '#f87171',
        yellow: '#fbbf24',
        blue: '#60a5fa',
        cyan: '#22d3ee',
      },
    });
    const fit = new FitAddon();
    xterm.loadAddon(fit);
    xterm.open(termEl);
    fit.fit();
    xterm.writeln('\x1b[90mMeetoOps Interactive Container Terminal (RBAC Audited)\x1b[0m');

    xterm.onData((data) => {
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: 'input', data }));
      }
    });

    const ro = new ResizeObserver(() => {
      fit.fit();
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: 'resize', cols: xterm.cols, rows: xterm.rows }));
      }
    });
    ro.observe(termEl);

    term = xterm;
    fitAddon = fit;

    return () => {
      disconnect();
      ro.disconnect();
      xterm.dispose();
      term = null;
      fitAddon = null;
    };
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <Terminal class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Terminal Kontainer Interaktif' : 'Container Terminal'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        Authenticated xterm.js PTY session via Docker exec (every session open is recorded in Audit Logs).
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <Select
        class="min-w-56"
        bind:value={selectedId}
        placeholder="Select running container…"
        emptyText="No running containers"
        options={containers.map((c) => ({
          value: c.id,
          label: c.name,
          description: `${c.image} · ${c.short_id}`,
        }))}
      />

      <Select
        class="w-32"
        searchable={false}
        bind:value={shell}
        options={[
          { value: '/bin/sh', label: '/bin/sh' },
          { value: '/bin/bash', label: '/bin/bash' },
          { value: '/bin/ash', label: '/bin/ash' },
        ]}
      />

      {#if connected}
        <button
          type="button"
          onclick={disconnect}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--danger)] text-white text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Unplug class="w-3.5 h-3.5" /> Disconnect
        </button>
      {:else}
        <button
          type="button"
          disabled={!selectedId || !canExec}
          onclick={connect}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-40"
        >
          <Plug class="w-3.5 h-3.5" /> Connect
        </button>
      {/if}

      <button
        type="button"
        onclick={() => term?.clear()}
        class="h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)] transition-colors"
        title="Clear Terminal"
      >
        <Trash2 class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>

  {#if !canExec}
    <div class="p-3 rounded-[var(--radius-md)] bg-[var(--warning-subtle)] border border-[var(--warning)]/40 text-[var(--warning)] text-xs">
      Your role does not have `terminal:exec` permission.
    </div>
  {/if}

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] border border-[var(--border-strong)] overflow-hidden bg-[#071426]">
    <div class="px-4 py-2 bg-[#0b192e] border-b border-[#1e3552] flex items-center justify-between text-[11px] text-[#cbd5e1]">
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full {connected ? 'bg-[#4ade80]' : 'bg-[#94a3b8]'}"></span>
        <span>{connected ? `Connected (${shell})` : 'Idle'}</span>
      </div>
      <span class="font-mono">xterm.js · PTY</span>
    </div>
    <div bind:this={termEl} class="w-full h-[64vh] p-2"></div>
  </div>
</div>
