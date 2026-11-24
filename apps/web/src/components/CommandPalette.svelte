<script lang="ts">
  import {
    Search,
    LayoutDashboard,
    Activity,
    Box,
    Layers,
    Terminal,
    Image,
    HardDrive,
    Network,
    ScrollText,
    Radar,
    Database,
    KeyRound,
    Server,
    Bell,
    Settings,
    CornerDownLeft,
  } from 'lucide-svelte';

  interface Props {
    open: boolean;
    onClose: () => void;
    onNavigate: (page: string) => void;
  }

  let { open, onClose, onNavigate }: Props = $props();
  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  const items = [
    { id: 'dashboard', label: 'Dashboard', group: 'Core', keywords: 'home overview metrics', icon: LayoutDashboard },
    { id: 'monitoring', label: 'Monitoring', group: 'Core', keywords: 'cpu ram disk network charts telemetry', icon: Activity },
    { id: 'alerts', label: 'Alert Center', group: 'Core', keywords: 'rules events webhook slack discord', icon: Bell },
    { id: 'servers', label: 'Servers & Agents', group: 'Core', keywords: 'nodes hosts remote agent token', icon: Server },
    { id: 'containers', label: 'Containers', group: 'Docker', keywords: 'docker ps start stop restart stats', icon: Box },
    { id: 'stacks', label: 'Compose Stacks', group: 'Docker', keywords: 'docker compose yaml deploy', icon: Layers },
    { id: 'projects', label: 'GitHub Projects & Deploy', group: 'Docker', keywords: 'github git clone pull deploy env compose', icon: Layers },
    { id: 'terminal', label: 'SSH & Container Terminal', group: 'Docker', keywords: 'ssh termius sudo exec shell bash sh xterm', icon: Terminal },
    { id: 'images', label: 'Images', group: 'Docker', keywords: 'docker pull registry', icon: Image },
    { id: 'volumes', label: 'Volumes', group: 'Docker', keywords: 'storage mount persistence', icon: HardDrive },
    { id: 'networks', label: 'Networks', group: 'Docker', keywords: 'bridge overlay subnet', icon: Network },
    { id: 'logs', label: 'Log Center', group: 'Docker', keywords: 'stdout stderr stream websocket', icon: ScrollText },
    { id: 'discovery', label: 'Service Discovery', group: 'Data', keywords: 'scan detect postgres mysql redis mongo', icon: Radar },
    { id: 'databases', label: 'Database Explorer', group: 'Data', keywords: 'sql postgres mysql mariadb mongodb csv json', icon: Database },
    { id: 'redis', label: 'Redis Explorer', group: 'Data', keywords: 'cache keys scan ttl hash list set', icon: KeyRound },
    { id: 'settings', label: 'Settings', group: 'Admin', keywords: 'users audit backup tokens port language theme', icon: Settings },
  ];

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (it) =>
        it.label.toLowerCase().includes(q) ||
        it.group.toLowerCase().includes(q) ||
        it.keywords.includes(q),
    );
  });

  $effect(() => {
    if (open) {
      query = '';
      selectedIndex = 0;
      setTimeout(() => inputEl?.focus(), 20);
    }
  });

  $effect(() => {
    if (selectedIndex >= filtered.length) {
      selectedIndex = Math.max(0, filtered.length - 1);
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = filtered.length ? (selectedIndex + 1) % filtered.length : 0;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = filtered.length ? (selectedIndex - 1 + filtered.length) % filtered.length : 0;
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      e.preventDefault();
      onNavigate(filtered[selectedIndex].id);
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div
    class="fixed inset-0 z-[105] flex items-start justify-center bg-black/55 px-4 pt-[12vh] backdrop-blur-sm"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') onClose();
    }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="w-full max-w-xl overflow-hidden rounded-2xl border border-[var(--border-strong)] bg-[var(--surface)] shadow-2xl">
      <div class="flex items-center gap-3 border-b border-[var(--border)] px-4 py-3">
        <Search class="h-4 w-4 shrink-0 text-[var(--muted-foreground)]" />
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          placeholder="Jump to page or module… (e.g. containers, monitoring, alerts, redis)"
          class="w-full bg-transparent text-sm text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)]"
        />
        <kbd class="rounded border border-[var(--border)] bg-[var(--surface-secondary)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--muted-foreground)]">
          ESC
        </kbd>
      </div>

      <div class="max-h-80 overflow-y-auto p-2">
        {#if filtered.length === 0}
          <div class="py-8 text-center text-sm text-[var(--muted-foreground)]">
            No matching modules found.
          </div>
        {:else}
          {#each filtered as item, idx (item.id)}
            {@const Icon = item.icon}
            <button
              type="button"
              onmouseenter={() => (selectedIndex = idx)}
              onclick={() => {
                onNavigate(item.id);
                onClose();
              }}
              class={`flex w-full items-center justify-between rounded-xl px-3 py-2.5 text-left text-sm transition-colors ${
                idx === selectedIndex
                  ? 'bg-[var(--primary-subtle)] text-[var(--primary)] font-medium'
                  : 'text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'
              }`}
            >
              <span class="flex items-center gap-3">
                <Icon class="h-4 w-4 shrink-0" />
                <span>{item.label}</span>
              </span>
              <span class="flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
                <span>{item.group}</span>
                {#if idx === selectedIndex}
                  <CornerDownLeft class="h-3.5 w-3.5 text-[var(--primary)]" />
                {/if}
              </span>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
