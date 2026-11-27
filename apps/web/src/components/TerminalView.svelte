<script lang="ts">
  import { untrack } from 'svelte';
  import { Terminal as XTerm } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';
  import { api, type ContainerSummaryDto, type SshHostDto } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import ExplorerTabs from './ui/ExplorerTabs.svelte';
  import {
    Terminal,
    Plug,
    Unplug,
    Trash2,
    AlertCircle,
    Plus,
    ShieldCheck,
    X,
    Server,
    Box,
    Loader2,
  } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canExec: boolean;
    currentUsername?: string;
    initialContainerId?: string | null;
  }

  let { lang, canExec, currentUsername = '', initialContainerId = null }: Props = $props();

  let mode = $state<'ssh' | 'container'>(untrack(() => (initialContainerId ? 'container' : 'ssh')));

  // Container Exec state
  let containers = $state<ContainerSummaryDto[]>([]);
  let selectedId = $state(untrack(() => initialContainerId ?? ''));
  let shell = $state('/bin/sh');

  // Host SSH (Termius) state
  let sshHosts = $state<SshHostDto[]>([]);
  let selectedSshId = $state('');
  let showAddSsh = $state(false);
  let sshName = $state('Local VPS');
  let sshHost = $state('127.0.0.1');
  let sshPort = $state(22);
  let sshUser = $state('root');
  let sshAuthType = $state<'password' | 'private_key'>('password');
  let sshPassword = $state('');
  let sshPrivateKey = $state('');
  let sshPassphrase = $state('');
  let sshUseSudo = $state(true);
  let savingSsh = $state(false);

  // 1-Click Provision Linux Sudo User modal
  let showProvision = $state(false);
  let provUsername = $state(untrack(() => currentUsername));
  let provPassword = $state('');
  let provDockerGroup = $state(true);
  let provisioning = $state(false);

  let connected = $state(false);
  let errorMsg = $state<string | null>(null);
  let termEl = $state<HTMLDivElement | null>(null);

  let term: XTerm | null = null;
  let fitAddon: FitAddon | null = null;
  let socket: WebSocket | null = null;

  const activeSshHost = $derived(sshHosts.find((h) => h.id === selectedSshId) ?? null);

  async function loadAll() {
    errorMsg = null;
    try {
      const [hostsRes, contRes] = await Promise.all([
        api.listSshHosts().catch(() => [] as SshHostDto[]),
        api.queryContainers({ all: false, state: 'running', per_page: 100 }).catch(() => ({ items: [] as ContainerSummaryDto[] })),
      ]);
      sshHosts = hostsRes;
      if (!selectedSshId && sshHosts.length > 0) {
        selectedSshId = sshHosts[0].id;
      }
      containers = contRes.items;
      if (!selectedId && containers.length > 0) {
        selectedId = containers[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load terminal targets.';
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
    if (!canExec) return;
    disconnect();
    errorMsg = null;

    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    let url = '';
    if (mode === 'ssh') {
      if (!selectedSshId) return;
      const cols = term?.cols ?? 100;
      const rows = term?.rows ?? 30;
      url = `${proto}//${window.location.host}/ws/v1/ssh/hosts/${encodeURIComponent(selectedSshId)}/terminal?cols=${cols}&rows=${rows}`;
    } else {
      if (!selectedId) return;
      url = `${proto}//${window.location.host}/ws/v1/docker/containers/${encodeURIComponent(selectedId)}/exec?shell=${encodeURIComponent(shell)}`;
    }

    const ws = new WebSocket(url);
    ws.onopen = () => {
      connected = true;
      term?.clear();
      if (mode === 'ssh' && activeSshHost) {
        term?.writeln(
          `\x1b[36mConnected via SSH to ${activeSshHost.username}@${activeSshHost.host}:${activeSshHost.port} (${activeSshHost.name})\x1b[0m\r\n`,
        );
      } else {
        term?.writeln(`\x1b[36mConnected to container ${selectedId.slice(0, 12)} (${shell})\x1b[0m\r\n`);
      }
      if (term && fitAddon) {
        fitAddon.fit();
        ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
      }
      term?.focus();
    };
    ws.onmessage = (ev) => {
      if (typeof ev.data === 'string') {
        term?.write(ev.data);
      }
    };
    ws.onclose = () => {
      connected = false;
      term?.writeln('\r\n\x1b[33m[Session closed]\x1b[0m');
    };
    ws.onerror = () => {
      errorMsg = 'WebSocket terminal connection failed.';
      connected = false;
    };
    socket = ws;
  }

  async function handleAddSsh(e: SubmitEvent) {
    e.preventDefault();
    savingSsh = true;
    errorMsg = null;
    try {
      const res = await api.createSshHost({
        name: sshName.trim(),
        host: sshHost.trim(),
        port: Number(sshPort) || 22,
        username: sshUser.trim(),
        auth_type: sshAuthType,
        password: sshAuthType === 'password' ? sshPassword : undefined,
        private_key_pem: sshAuthType === 'private_key' ? sshPrivateKey : undefined,
        passphrase: sshAuthType === 'private_key' ? sshPassphrase || undefined : undefined,
        use_sudo: sshUseSudo,
      });
      await loadAll();
      selectedSshId = res.host.id;
      showAddSsh = false;
      sshPassword = '';
      sshPrivateKey = '';
      sshPassphrase = '';
      if (res.verified) {
        toast.success('SSH host connected', res.banner ?? res.host.name);
      } else {
        toast.warning('SSH host saved (verify failed)', res.verify_error ?? 'Check credentials');
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to save SSH host.';
      errorMsg = msg;
      toast.error('SSH save failed', msg);
    } finally {
      savingSsh = false;
    }
  }

  async function handleProvisionUser(e: SubmitEvent) {
    e.preventDefault();
    if (!selectedSshId || !provPassword) return;
    provisioning = true;
    errorMsg = null;
    try {
      const res = await api.provisionSshUser(selectedSshId, {
        linux_username: provUsername.trim() || undefined,
        password: provPassword,
        add_docker_group: provDockerGroup,
      });
      toast.success(
        'Linux sudo user provisioned',
        `Switched SSH profile to '${res.linux_username}' (sudo + docker).`,
      );
      showProvision = false;
      provPassword = '';
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to provision Linux user.';
      errorMsg = msg;
      toast.error('Provisioning failed', msg);
    } finally {
      provisioning = false;
    }
  }

  async function handleDeleteSsh() {
    if (!activeSshHost) return;
    const ok = await confirmDialog({
      title: 'Delete SSH Host',
      message: 'Remove this saved SSH profile and its encrypted credentials?',
      targetName: `${activeSshHost.name} (${activeSshHost.username}@${activeSshHost.host})`,
      confirmText: 'Delete Host',
      variant: 'danger',
    });
    if (!ok) return;
    disconnect();
    await api.deleteSshHost(activeSshHost.id);
    toast.success('SSH host deleted', activeSshHost.name);
    selectedSshId = '';
    await loadAll();
  }

  $effect(() => {
    loadAll();
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
    xterm.writeln('\x1b[90mMeetoOps Terminal (Host SSH + Container Exec · RBAC Audited)\x1b[0m');

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
        {lang === 'id' ? 'Terminal SSH & Kontainer' : 'SSH & Container Terminal'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        Built-in Host SSH client (Termius replacement with auto-sudo user provisioning) & interactive Docker exec.
      </p>
    </div>
  </div>

  <!-- Connected File-Explorer Mode Tabs -->
  <div class="rounded-[var(--radius-lg)] bg-[var(--surface)] border border-[var(--border)] shadow-[var(--shadow-sm)] overflow-hidden">
    <ExplorerTabs
      framed={false}
      bind:active={mode}
      onChange={() => disconnect()}
      tabs={[
        { id: 'ssh', label: 'Host SSH', icon: Server, badge: sshHosts.length > 0 ? sshHosts.length : undefined },
        { id: 'container', label: 'Container Exec', icon: Box, badge: containers.length > 0 ? containers.length : undefined },
      ]}
    />
    <div class="p-3">
  <!-- Action Toolbar -->
  <div class="flex flex-wrap items-center justify-between gap-2">
    <div class="flex flex-wrap items-center gap-2">
      {#if mode === 'ssh'}
        <Select
          class="min-w-60"
          bind:value={selectedSshId}
          placeholder="Select SSH host…"
          emptyText="No SSH hosts yet"
          options={sshHosts.map((h) => ({
            value: h.id,
            label: h.name,
            description: `${h.username}@${h.host}:${h.port}${h.provisioned_user ? ' · sudo' : ''}`,
          }))}
        />

        <button
          type="button"
          onclick={() => (showAddSsh = !showAddSsh)}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs font-medium inline-flex items-center gap-1.5 hover:bg-[var(--surface-elevated)]"
        >
          <Plus class="w-3.5 h-3.5" /> Add SSH Host
        </button>

        {#if activeSshHost}
          <button
            type="button"
            onclick={() => (showProvision = !showProvision)}
            class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--success-subtle)] border border-[var(--success)]/40 text-[var(--success)] text-xs font-medium inline-flex items-center gap-1.5 hover:opacity-90"
            title="Create Linux sudo user matching your MeetoOps account"
          >
            <ShieldCheck class="w-3.5 h-3.5" /> Provision Sudo User
          </button>
        {/if}
      {:else}
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
      {/if}
    </div>

    <div class="flex items-center gap-2">
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
          disabled={!canExec || (mode === 'ssh' ? !selectedSshId : !selectedId)}
          onclick={connect}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-40"
        >
          <Plug class="w-3.5 h-3.5" /> Connect
        </button>
      {/if}

      {#if mode === 'ssh' && activeSshHost}
        <button
          type="button"
          onclick={handleDeleteSsh}
          class="h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
          title="Delete SSH Host"
        >
          <Trash2 class="w-3.5 h-3.5" />
        </button>
      {/if}

      <button
        type="button"
        onclick={() => term?.clear()}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
      >
        Clear
      </button>
    </div>
  </div>
    </div>
  </div>

  {#if showAddSsh}
    <form
      onsubmit={handleAddSsh}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-4"
    >
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-sm font-semibold text-[var(--foreground)]">Add SSH Host Connection</h2>
          <p class="text-xs text-[var(--muted-foreground)]">
            Credentials are encrypted with AES-256-GCM. For this VPS itself, use <code class="font-mono">127.0.0.1</code> port <code class="font-mono">22</code>.
          </p>
        </div>
        <button type="button" onclick={() => (showAddSsh = false)} class="text-[var(--muted-foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Label</span>
          <input type="text" required bind:value={sshName} placeholder="Production VPS" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Host / IP</span>
          <input type="text" required bind:value={sshHost} placeholder="127.0.0.1" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Port</span>
          <input type="number" required bind:value={sshPort} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Username</span>
          <input type="text" required bind:value={sshUser} placeholder="root" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3 items-end">
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Auth Method</span>
          <Select
            class="w-full"
            searchable={false}
            bind:value={sshAuthType}
            options={[
              { value: 'password', label: 'Password' },
              { value: 'private_key', label: 'SSH Private Key (PEM)' },
            ]}
          />
        </div>
        {#if sshAuthType === 'password'}
          <label class="md:col-span-2 space-y-1">
            <span class="text-[11px] text-[var(--muted-foreground)]">SSH Password</span>
            <input type="password" required bind:value={sshPassword} placeholder="••••••••" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
          </label>
        {:else}
          <label class="md:col-span-2 space-y-1">
            <span class="text-[11px] text-[var(--muted-foreground)]">Passphrase (optional)</span>
            <input type="password" bind:value={sshPassphrase} placeholder="Optional key passphrase" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
          </label>
        {/if}
      </div>

      {#if sshAuthType === 'private_key'}
        <label class="block space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Private Key PEM</span>
          <textarea rows="4" required bind:value={sshPrivateKey} placeholder="-----BEGIN OPENSSH PRIVATE KEY-----" class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
      {/if}

      <div class="flex items-center justify-between">
        <Checkbox bind:checked={sshUseSudo} label="Enable sudo workflow for non-root user" />
        <button
          type="submit"
          disabled={savingSsh}
          class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
        >
          {#if savingSsh}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
          Save & Verify SSH
        </button>
      </div>
    </form>
  {/if}

  {#if showProvision && activeSshHost}
    <form
      onsubmit={handleProvisionUser}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--success)]/45 space-y-4"
    >
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-sm font-semibold text-[var(--foreground)] flex items-center gap-2">
            <ShieldCheck class="w-4 h-4 text-[var(--success)]" />
            Provision Non-Root Linux Sudo User on {activeSshHost.name}
          </h2>
          <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
            Creates a Linux user matching your MeetoOps username, adds it to <code class="font-mono">sudo</code> (enter password once per session for admin actions) and <code class="font-mono">docker</code>, then switches this SSH profile automatically.
          </p>
        </div>
        <button type="button" onclick={() => (showProvision = false)} class="text-[var(--muted-foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Linux Username (defaults to your MeetoOps user)</span>
          <input type="text" bind:value={provUsername} placeholder="admin_osce" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Sudo Password (min 8 chars)</span>
          <input type="password" required minlength="8" bind:value={provPassword} placeholder="Strong password for sudo" class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
      </div>

      <div class="flex items-center justify-between">
        <Checkbox bind:checked={provDockerGroup} label="Add user to 'docker' group (run docker compose cleanly)" />
        <button
          type="submit"
          disabled={provisioning}
          class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--success)] text-white text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
        >
          {#if provisioning}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
          Create User & Switch Profile
        </button>
      </div>
    </form>
  {/if}

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] border border-[var(--border-strong)] overflow-hidden bg-[#071426] p-2">
    <div bind:this={termEl} class="w-full h-[65vh]"></div>
  </div>
</div>
