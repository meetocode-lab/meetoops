<script lang="ts">
  import { api, type ComposeStackDto } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { toast } from '../lib/toast.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import { FileCode2, RefreshCw, Play, Square, RotateCw, AlertCircle, Search, Plus, X, Download, Copy } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
  }

  let { lang, canOperate }: Props = $props();

  let stacks = $state<ComposeStackDto[]>([]);
  let loading = $state(true);
  let busyProject = $state<string | null>(null);
  let search = $state('');
  let errorMsg = $state<string | null>(null);
  let showYamlDrawer = $state(false);
  let yamlProjectName = $state('my-app-stack');
  let yamlContent = $state(`services:
  web:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "8080:80"
`);

  const filtered = $derived(
    search.trim()
      ? stacks.filter((s) => s.project_name.toLowerCase().includes(search.toLowerCase()))
      : stacks,
  );

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      stacks = await api.listStacks();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load Compose stacks.';
    } finally {
      loading = false;
    }
  }

  async function runAction(project: string, action: 'start' | 'stop' | 'restart') {
    busyProject = project;
    errorMsg = null;
    try {
      await api.stackAction(project, action);
      toast.success(`Stack ${action}ed`, project);
      await load();
    } catch (err) {
      const msg = err instanceof Error ? err.message : `Failed to ${action} stack.`;
      errorMsg = msg;
      toast.error(`Stack ${action} failed`, msg);
    } finally {
      busyProject = null;
    }
  }

  function downloadComposeYaml() {
    const blob = new Blob([yamlContent], { type: 'text/yaml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${yamlProjectName.trim() || 'docker-compose'}.yml`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('Compose YAML downloaded', `${yamlProjectName.trim() || 'docker-compose'}.yml`);
  }

  async function copyComposeCmd() {
    const cmd = `docker compose -p ${yamlProjectName.trim() || 'my-stack'} -f ${yamlProjectName.trim() || 'docker-compose'}.yml up -d`;
    await navigator.clipboard.writeText(cmd);
    toast.info('Deploy command copied to clipboard');
  }

  $effect(() => {
    load();
  });
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <FileCode2 class="w-5 h-5 text-[var(--primary)]" />
        Compose Stacks
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Deteksi otomatis proyek Docker Compose dari label kontainer aktif beserta kontrol start/stop/restart.'
          : 'Auto-detected Docker Compose projects from live container labels with stack-level lifecycle controls.'}
      </p>
    </div>

    <div class="flex items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder="Search stack…"
          bind:value={search}
          class="pl-8 pr-3 py-1.5 w-52 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
        />
      </div>
      {#if canOperate}
        <button
          type="button"
          onclick={() => (showYamlDrawer = !showYamlDrawer)}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Plus class="w-3.5 h-3.5" /> Compose YAML
        </button>
      {/if}
      <button
        type="button"
        onclick={load}
        disabled={loading}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5"
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" /> Refresh
      </button>
    </div>
  </div>

  {#if showYamlDrawer}
    <div class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-3">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-sm font-semibold text-[var(--foreground)]">Compose Stack Authoring</h2>
          <p class="text-xs text-[var(--muted-foreground)]">
            Draft a Docker Compose stack YAML, export it, and launch with <code class="font-mono">docker compose up -d</code> — MeetoOps automatically detects the live stack via container labels.
          </p>
        </div>
        <button type="button" onclick={() => (showYamlDrawer = false)} class="p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>
      <div class="flex items-center gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Project Name</span>
          <input
            type="text"
            bind:value={yamlProjectName}
            class="block px-3 py-1.5 w-60 rounded bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"
          />
        </label>
      </div>
      <textarea
        rows="10"
        bind:value={yamlContent}
        class="w-full p-3 rounded bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)]"
      ></textarea>
      <div class="flex justify-end gap-2">
        <button
          type="button"
          onclick={copyComposeCmd}
          class="px-3 py-1.5 rounded bg-[var(--surface-secondary)] border border-[var(--border)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Copy class="w-3.5 h-3.5" /> Copy Deploy Command
        </button>
        <button
          type="button"
          onclick={downloadComposeYaml}
          class="px-3 py-1.5 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Download class="w-3.5 h-3.5" /> Download YAML
        </button>
      </div>
    </div>
  {/if}

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
    {#if loading && stacks.length === 0}
      <TableSkeleton rows={4} columns={5} />
    {:else if filtered.length === 0}
      <div class="p-10 text-center space-y-2">
        <FileCode2 class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">
          No Docker Compose stacks detected on this host.
        </div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse text-xs">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th class="px-4 py-2.5 font-medium">Project</th>
            <th class="px-4 py-2.5 font-medium">Services</th>
            <th class="px-4 py-2.5 font-medium">Status</th>
            <th class="px-4 py-2.5 font-medium">Working Dir</th>
            <th class="px-4 py-2.5 font-medium text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)]">
          {#each filtered as st}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-2.5 font-mono font-semibold text-[var(--foreground)]">{st.project_name}</td>
              <td class="px-4 py-2.5 text-[var(--muted-foreground)]">
                {st.services.map((s) => s.service_name).join(', ')}
              </td>
              <td class="px-4 py-2.5 font-mono tabular-nums">
                <span class="px-2 py-0.5 rounded-full text-[11px] {st.running_services === st.total_services ? 'bg-[var(--success-subtle)] text-[var(--success)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)]'}">
                  {st.running_services} / {st.total_services} running
                </span>
              </td>
              <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] truncate max-w-[260px]">
                {st.working_dir ?? '—'}
              </td>
              <td class="px-4 py-2.5 text-right whitespace-nowrap">
                {#if canOperate}
                  <div class="inline-flex items-center gap-1">
                    <button
                      type="button"
                      disabled={busyProject === st.project_name}
                      onclick={() => runAction(st.project_name, 'start')}
                      class="p-1.5 rounded hover:bg-[var(--success-subtle)] text-[var(--success)] disabled:opacity-40"
                      title="Start Stack"
                    >
                      <Play class="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      disabled={busyProject === st.project_name}
                      onclick={() => runAction(st.project_name, 'restart')}
                      class="p-1.5 rounded hover:bg-[var(--primary-subtle)] text-[var(--primary)] disabled:opacity-40"
                      title="Restart Stack"
                    >
                      <RotateCw class="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      disabled={busyProject === st.project_name}
                      onclick={() => runAction(st.project_name, 'stop')}
                      class="p-1.5 rounded hover:bg-[var(--danger-subtle)] text-[var(--danger)] disabled:opacity-40"
                      title="Stop Stack"
                    >
                      <Square class="w-3.5 h-3.5" />
                    </button>
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
