<script lang="ts">
  import {
    api,
    type ProjectDeploymentDto,
    type ProjectDto,
    type SshHostDto,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import {
    GitBranch,
    Plus,
    Rocket,
    RefreshCw,
    RotateCw,
    Square,
    Trash2,
    AlertCircle,
    Settings2,
    ScrollText,
    X,
    Loader2,
    CheckCircle2,
    ArrowUpCircle,
    FolderSearch,
  } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canOperate: boolean;
    canDelete: boolean;
    onOpenTerminal?: () => void;
  }

  let { lang, canOperate, canDelete, onOpenTerminal }: Props = $props();

  let projects = $state<ProjectDto[]>([]);
  let sshHosts = $state<SshHostDto[]>([]);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let busyProjectId = $state<string | null>(null);
  let checkingId = $state<string | null>(null);

  // Scan Existing Repos Drawer
  let showScan = $state(false);
  let scanHostId = $state('');
  let scanBaseDir = $state('/home/app');
  let scanning = $state(false);

  // Create Project Drawer
  let showCreate = $state(false);
  let pName = $state('');
  let pSshHostId = $state('');
  let pRepoUrl = $state('');
  let pBranch = $state('main');
  let pDeployPath = $state('/home/app/');
  let pDeployCmd = $state('docker compose up -d --build');
  let pGitToken = $state('');
  let pEnvContent = $state('');
  let pComposeOverride = $state('');
  let creating = $state(false);

  // Edit Config / .env Drawer
  let editingProject = $state<ProjectDto | null>(null);
  let editBranch = $state('');
  let editCmd = $state('');
  let editCompose = $state('');
  let editGitToken = $state('');
  let editEnv = $state('');
  let savingEdit = $state(false);

  // Deployment Logs Drawer
  let logProject = $state<ProjectDto | null>(null);
  let deployments = $state<ProjectDeploymentDto[]>([]);
  let activeDeployLog = $state<ProjectDeploymentDto | null>(null);
  let loadingLogs = $state(false);

  async function loadAll() {
    loading = true;
    errorMsg = null;
    try {
      const [projs, hosts] = await Promise.all([api.listProjects(), api.listSshHosts()]);
      projects = projs;
      sshHosts = hosts;
      if (!pSshHostId && hosts.length > 0) {
        pSshHostId = hosts[0].id;
      }
      if (!scanHostId && hosts.length > 0) {
        scanHostId = hosts[0].id;
      }
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load projects.';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadAll();
  });

  async function handleScanExisting(e: SubmitEvent) {
    e.preventDefault();
    if (!scanHostId) {
      toast.warning('Select an SSH host first');
      return;
    }
    scanning = true;
    errorMsg = null;
    try {
      const res = await api.scanExistingProjects(scanHostId, scanBaseDir.trim() || '/home/app');
      projects = res.projects;
      if (res.imported_count > 0) {
        toast.success(
          `Imported ${res.imported_count} existing repo(s)`,
          `Found ${res.found_count} Git repo(s) in ${res.scanned_dir}`,
        );
      } else if (res.found_count > 0) {
        toast.info(
          'All repos already imported',
          `Found ${res.found_count} Git repo(s) in ${res.scanned_dir}`,
        );
      } else {
        toast.warning('No Git repositories found', `Scanned ${res.scanned_dir} (depth 3)`);
      }
      showScan = false;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to scan existing folders.';
      errorMsg = msg;
      toast.error('Scan failed', msg);
    } finally {
      scanning = false;
    }
  }

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault();
    if (!pSshHostId) {
      toast.warning('Select an SSH host first', 'Add an SSH host in Terminal -> Host SSH.');
      return;
    }
    creating = true;
    errorMsg = null;
    try {
      const created = await api.createProject({
        name: pName.trim(),
        ssh_host_id: pSshHostId,
        repo_url: pRepoUrl.trim(),
        branch: pBranch.trim() || 'main',
        deploy_path: pDeployPath.trim(),
        deploy_command: pDeployCmd.trim() || 'docker compose up -d --build',
        compose_override: pComposeOverride.trim() || undefined,
        git_token: pGitToken.trim() || undefined,
        env_content: pEnvContent.trim() || undefined,
      });
      toast.success('Project created', created.name);
      showCreate = false;
      pName = '';
      pRepoUrl = '';
      pBranch = 'main';
      pDeployPath = '/home/app/';
      pDeployCmd = 'docker compose up -d --build';
      pGitToken = '';
      pEnvContent = '';
      pComposeOverride = '';
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create project.';
      errorMsg = msg;
      toast.error('Create failed', msg);
    } finally {
      creating = false;
    }
  }

  async function openEdit(p: ProjectDto) {
    editingProject = p;
    editBranch = p.branch;
    editCmd = p.deploy_command;
    editCompose = p.compose_override ?? '';
    editGitToken = '';
    editEnv = '';
    if (p.has_env_vars) {
      try {
        const res = await api.revealProjectEnv(p.id);
        editEnv = res.env_content;
      } catch {
        // leave empty if user lacks reveal permission
      }
    }
  }

  async function handleSaveEdit(e: SubmitEvent) {
    e.preventDefault();
    if (!editingProject) return;
    savingEdit = true;
    errorMsg = null;
    try {
      await api.updateProject(editingProject.id, {
        branch: editBranch.trim(),
        deploy_command: editCmd.trim(),
        compose_override: editCompose,
        git_token: editGitToken.trim() || undefined,
        env_content: editEnv,
      });
      toast.success('Project configuration updated', editingProject.name);
      editingProject = null;
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to update project.';
      errorMsg = msg;
      toast.error('Update failed', msg);
    } finally {
      savingEdit = false;
    }
  }

  async function handleCheckRemote(p: ProjectDto) {
    checkingId = p.id;
    errorMsg = null;
    try {
      const updated = await api.checkProjectRemote(p.id);
      if (updated.update_available) {
        toast.info('Update available on GitHub', `${p.name} (${updated.remote_head_commit})`);
      } else {
        toast.success('Up to date with GitHub', `${p.name} (${updated.remote_head_commit ?? '—'})`);
      }
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to check remote repository.';
      errorMsg = msg;
      toast.error('Remote check failed', msg);
    } finally {
      checkingId = null;
    }
  }

  async function handleDeploy(p: ProjectDto) {
    busyProjectId = p.id;
    errorMsg = null;
    toast.info('Deploying project…', `${p.name} (${p.branch})`);
    try {
      const dep = await api.deployProject(p.id);
      if (dep.status === 'success') {
        toast.success('Deployment succeeded', `${p.name} @ ${dep.commit_hash ?? 'latest'}`);
      } else {
        toast.error('Deployment failed', 'Check deployment logs for details.');
      }
      logProject = p;
      activeDeployLog = dep;
      deployments = await api.listProjectDeployments(p.id);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Deployment failed.';
      errorMsg = msg;
      toast.error('Deployment error', msg);
    } finally {
      busyProjectId = null;
    }
  }

  async function handleAction(p: ProjectDto, action: 'restart' | 'stop' | 'logs') {
    busyProjectId = p.id;
    errorMsg = null;
    try {
      const dep = await api.projectAction(p.id, action);
      toast.success(`Project ${action} completed`, p.name);
      logProject = p;
      activeDeployLog = dep;
      deployments = await api.listProjectDeployments(p.id);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : `Failed to ${action} project.`;
      errorMsg = msg;
      toast.error(`Action failed`, msg);
    } finally {
      busyProjectId = null;
    }
  }

  async function openLogs(p: ProjectDto) {
    logProject = p;
    loadingLogs = true;
    try {
      deployments = await api.listProjectDeployments(p.id);
      activeDeployLog = deployments[0] ?? null;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load deployment logs.';
    } finally {
      loadingLogs = false;
    }
  }

  async function handleDelete(p: ProjectDto) {
    const ok = await confirmDialog({
      title: 'Delete Project',
      message: 'Remove this project and its encrypted GitHub token / .env from MeetoOps? Files on the remote server are not deleted.',
      targetName: p.name,
      confirmText: 'Delete Project',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteProject(p.id);
      toast.success('Project removed', p.name);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to delete project.';
      errorMsg = msg;
      toast.error('Delete failed', msg);
    }
  }
</script>

<div class="w-full space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <GitBranch class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Proyek GitHub & Deploy' : 'GitHub Projects & Deploy'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Clone, pantau commit remote GitHub, kelola .env terenkripsi, dan jalankan docker compose deploy langsung dari GUI.'
          : 'Clone repositories, monitor remote GitHub commits, manage encrypted .env files, and deploy via SSH.'}
      </p>
    </div>

    <div class="flex items-center gap-2">
      {#if canOperate}
        <button
          type="button"
          onclick={() => (showScan = !showScan)}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <FolderSearch class="w-3.5 h-3.5 text-[var(--primary)]" />
          {lang === 'id' ? 'Scan Folder Existing' : 'Scan Existing Repos'}
        </button>
        <button
          type="button"
          onclick={() => (showCreate = !showCreate)}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5"
        >
          <Plus class="w-3.5 h-3.5" /> New Project
        </button>
      {/if}
      <button
        type="button"
        onclick={loadAll}
        disabled={loading}
        class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs inline-flex items-center gap-1.5"
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" /> Refresh
      </button>
    </div>
  </div>

  {#if sshHosts.length === 0 && !loading}
    <div class="p-4 rounded-[var(--radius-md)] bg-[var(--warning-subtle)] border border-[var(--warning)]/40 text-xs flex flex-wrap items-center justify-between gap-3">
      <div class="text-[var(--foreground)]">
        <strong>No SSH host configured yet.</strong> Add your VPS SSH connection first in <strong>Terminal → Host SSH</strong> so MeetoOps can clone & deploy projects.
      </div>
      {#if onOpenTerminal}
        <button
          type="button"
          onclick={onOpenTerminal}
          class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium"
        >
          Open Host SSH Setup
        </button>
      {/if}
    </div>
  {/if}

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if showScan}
    <form
      onsubmit={handleScanExisting}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-3"
    >
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-sm font-semibold text-[var(--foreground)] flex items-center gap-2">
            <FolderSearch class="w-4 h-4 text-[var(--primary)]" />
            {lang === 'id' ? 'Scan Folder GitHub Existing di Server' : 'Scan Existing GitHub Folders on Server'}
          </h2>
          <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
            Automatically detects existing <code class="font-mono">.git</code> repositories (origin URL, active branch, HEAD commit, and <code class="font-mono">.env</code>) up to 3 levels deep.
          </p>
        </div>
        <button type="button" onclick={() => (showScan = false)} class="text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-12 gap-3 items-end">
        <div class="md:col-span-4 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Target SSH Host</span>
          <Select
            class="w-full"
            bind:value={scanHostId}
            placeholder="Select SSH host…"
            options={sshHosts.map((h) => ({ value: h.id, label: h.name, description: `${h.username}@${h.host}:${h.port}` }))}
          />
        </div>
        <label class="md:col-span-5 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Base Directory on Server</span>
          <input
            type="text"
            required
            bind:value={scanBaseDir}
            placeholder="/home/app"
            class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"
          />
        </label>
        <button
          type="submit"
          disabled={scanning || !scanHostId}
          class="md:col-span-3 h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center justify-center gap-1.5 disabled:opacity-50"
        >
          {#if scanning}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<FolderSearch class="w-3.5 h-3.5" />{/if}
          {scanning ? 'Scanning…' : 'Scan & Import'}
        </button>
      </div>
    </form>
  {/if}

  {#if showCreate}
    <form
      onsubmit={handleCreate}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-4"
    >
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-[var(--foreground)]">Register New GitHub Project</h2>
        <button type="button" onclick={() => (showCreate = false)} class="text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Project Name</span>
          <input type="text" required placeholder="my-saas-api" bind:value={pName} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Target SSH Host</span>
          <Select
            class="w-full"
            bind:value={pSshHostId}
            placeholder="Select SSH host…"
            options={sshHosts.map((h) => ({ value: h.id, label: h.name, description: `${h.username}@${h.host}:${h.port}` }))}
          />
        </div>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Branch</span>
          <input type="text" required placeholder="main" bind:value={pBranch} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="md:col-span-2 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">GitHub Repository URL (HTTPS)</span>
          <input type="url" required placeholder="https://github.com/org/repo.git" bind:value={pRepoUrl} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">GitHub PAT (optional, private repos)</span>
          <input type="password" placeholder="ghp_xxxxxxxx" bind:value={pGitToken} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Deploy Directory on Server</span>
          <input type="text" required placeholder="/home/app/my-saas-api" bind:value={pDeployPath} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="md:col-span-2 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Deploy Command</span>
          <input type="text" required bind:value={pDeployCmd} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">.env File Content (encrypted with AES-256-GCM, optional)</span>
          <textarea rows="4" bind:value={pEnvContent} placeholder="NODE_ENV=production&#10;PORT=3000" class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Custom docker-compose.yml Override (optional)</span>
          <textarea rows="4" bind:value={pComposeOverride} placeholder="Leave blank to use repository's docker-compose.yml" class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
      </div>

      <div class="flex justify-end gap-2">
        <button type="button" onclick={() => (showCreate = false)} class="h-[34px] px-3 rounded-[var(--radius-md)] border border-[var(--border-strong)] text-xs">
          Cancel
        </button>
        <button type="submit" disabled={creating} class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50">
          {#if creating}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
          Save Project
        </button>
      </div>
    </form>
  {/if}

  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
    {#if loading && projects.length === 0}
      <TableSkeleton rows={4} columns={6} />
    {:else if projects.length === 0}
      <div class="p-10 text-center space-y-2">
        <GitBranch class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
        <div class="text-xs text-[var(--muted-foreground)]">
          No GitHub projects registered yet. Click <strong>New Project</strong> to deploy directly from GitHub via GUI.
        </div>
      </div>
    {:else}
      <table class="w-full text-left border-collapse text-xs">
        <thead>
          <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th class="px-4 py-2.5 font-medium">Project / Repo</th>
            <th class="px-4 py-2.5 font-medium">Target Server</th>
            <th class="px-4 py-2.5 font-medium">Git Status</th>
            <th class="px-4 py-2.5 font-medium">Deploy Status</th>
            <th class="px-4 py-2.5 font-medium text-right">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)]">
          {#each projects as p}
            <tr class="hover:bg-[var(--surface-elevated)]">
              <td class="px-4 py-3">
                <div class="font-semibold text-[var(--foreground)] flex items-center gap-2">
                  <span>{p.name}</span>
                  <span class="px-1.5 py-0.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[10px]">
                    {p.branch}
                  </span>
                  {#if p.has_env_vars}
                    <span class="px-1.5 py-0.5 rounded bg-[var(--surface-secondary)] text-[var(--muted-foreground)] font-mono text-[10px]">.env</span>
                  {/if}
                </div>
                <div class="font-mono text-[11px] text-[var(--muted-foreground)] truncate max-w-[320px] mt-0.5">{p.repo_url}</div>
                <div class="font-mono text-[10px] text-[var(--muted-foreground)]">{p.deploy_path}</div>
              </td>
              <td class="px-4 py-3 text-[var(--foreground)]">{p.ssh_host_name}</td>
              <td class="px-4 py-3 font-mono text-[11px]">
                <div class="flex items-center gap-1.5">
                  <span class="text-[var(--muted-foreground)]">Local:</span>
                  <span class="text-[var(--foreground)]">{p.last_deployed_commit ?? '—'}</span>
                </div>
                {#if p.update_available}
                  <div class="inline-flex items-center gap-1 mt-1 px-1.5 py-0.5 rounded bg-[var(--warning-subtle)] text-[var(--warning)] text-[10px] font-sans font-medium">
                    <ArrowUpCircle class="w-3 h-3" /> Update: {p.remote_head_commit}
                  </div>
                {:else if p.remote_head_commit}
                  <div class="inline-flex items-center gap-1 mt-1 text-[var(--success)] text-[10px] font-sans">
                    <CheckCircle2 class="w-3 h-3" /> Synced ({p.remote_head_commit})
                  </div>
                {/if}
              </td>
              <td class="px-4 py-3">
                <span
                  class="px-2 py-0.5 rounded-full text-[11px] font-medium capitalize {p.last_status === 'running'
                    ? 'bg-[var(--success-subtle)] text-[var(--success)]'
                    : p.last_status === 'failed'
                      ? 'bg-[var(--danger-subtle)] text-[var(--danger)]'
                      : p.last_status === 'deploying'
                        ? 'bg-[var(--info-subtle)] text-[var(--info)]'
                        : 'bg-[var(--surface-secondary)] text-[var(--muted-foreground)]'}"
                >
                  {p.last_status}
                </span>
                {#if p.last_deployed_at}
                  <div class="text-[10px] text-[var(--muted-foreground)] mt-1 tabular-nums">
                    {new Date(p.last_deployed_at).toLocaleString()}
                  </div>
                {/if}
              </td>
              <td class="px-4 py-3 text-right whitespace-nowrap">
                <div class="inline-flex items-center gap-1">
                  <button
                    type="button"
                    disabled={checkingId === p.id}
                    onclick={() => handleCheckRemote(p)}
                    title="Check GitHub Remote Commits"
                    class="p-1.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--info)] disabled:opacity-40"
                  >
                    <RefreshCw class="w-3.5 h-3.5 {checkingId === p.id ? 'animate-spin' : ''}" />
                  </button>
                  {#if canOperate}
                    <button
                      type="button"
                      disabled={busyProjectId === p.id}
                      onclick={() => handleDeploy(p)}
                      title="Git Pull & Deploy"
                      class="px-2.5 py-1 rounded bg-[var(--primary)] text-[var(--primary-foreground)] text-[11px] font-medium inline-flex items-center gap-1 disabled:opacity-40"
                    >
                      {#if busyProjectId === p.id}<Loader2 class="w-3 h-3 animate-spin" />{:else}<Rocket class="w-3 h-3" />{/if}
                      Deploy
                    </button>
                    <button
                      type="button"
                      disabled={busyProjectId === p.id}
                      onclick={() => handleAction(p, 'restart')}
                      title="Restart Compose"
                      class="p-1.5 rounded hover:bg-[var(--primary-subtle)] text-[var(--primary)] disabled:opacity-40"
                    >
                      <RotateCw class="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      disabled={busyProjectId === p.id}
                      onclick={() => handleAction(p, 'stop')}
                      title="Stop (docker compose down)"
                      class="p-1.5 rounded hover:bg-[var(--danger-subtle)] text-[var(--danger)] disabled:opacity-40"
                    >
                      <Square class="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      onclick={() => openEdit(p)}
                      title="Edit Config & .env"
                      class="p-1.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                    >
                      <Settings2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                  <button
                    type="button"
                    onclick={() => openLogs(p)}
                    title="Deployment Logs"
                    class="p-1.5 rounded hover:bg-[var(--surface-secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                  >
                    <ScrollText class="w-3.5 h-3.5" />
                  </button>
                  {#if canDelete}
                    <button
                      type="button"
                      onclick={() => handleDelete(p)}
                      title="Delete Project"
                      class="p-1.5 rounded hover:bg-[var(--danger-subtle)] text-[var(--danger)]"
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
  </div>

  {#if editingProject}
    <form
      onsubmit={handleSaveEdit}
      class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-4"
    >
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-[var(--foreground)]">Configure {editingProject.name}</h2>
        <button type="button" onclick={() => (editingProject = null)} class="text-[var(--muted-foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Branch</span>
          <input type="text" bind:value={editBranch} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Deploy Command</span>
          <input type="text" bind:value={editCmd} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Update GitHub PAT (leave blank to keep)</span>
          <input type="password" bind:value={editGitToken} placeholder="ghp_..." class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">.env Content (AES-256-GCM Encrypted)</span>
          <textarea rows="6" bind:value={editEnv} class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">docker-compose.yml Override (optional)</span>
          <textarea rows="6" bind:value={editCompose} class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
      </div>
      <div class="flex justify-end gap-2">
        <button type="button" onclick={() => (editingProject = null)} class="h-[34px] px-3 rounded-[var(--radius-md)] border border-[var(--border-strong)] text-xs">Cancel</button>
        <button type="submit" disabled={savingEdit} class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium">
          {savingEdit ? 'Saving…' : 'Save Configuration'}
        </button>
      </div>
    </form>
  {/if}

  {#if logProject}
    <div class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] space-y-3">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-[var(--foreground)]">Deployment Logs · {logProject.name}</h2>
        <button type="button" onclick={() => (logProject = null)} class="text-[var(--muted-foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>
      {#if loadingLogs}
        <TableSkeleton rows={3} columns={3} />
      {:else if deployments.length === 0}
        <div class="text-xs text-[var(--muted-foreground)] py-4">No deployments recorded yet.</div>
      {:else}
        <div class="flex flex-wrap gap-1.5">
          {#each deployments as d}
            <button
              type="button"
              onclick={() => (activeDeployLog = d)}
              class="px-2.5 py-1 rounded text-[11px] font-mono border {activeDeployLog?.id === d.id
                ? 'border-[var(--primary)] bg-[var(--primary-subtle)] text-[var(--primary)]'
                : 'border-[var(--border)] bg-[var(--background)] text-[var(--muted-foreground)]'}"
            >
              {d.action} · {d.status} · {new Date(d.started_at).toLocaleTimeString()}
            </button>
          {/each}
        </div>
        {#if activeDeployLog}
          <pre class="p-3.5 rounded-[var(--radius-md)] bg-[#071426] text-[#e2e8f0] font-mono text-xs overflow-x-auto max-h-96 whitespace-pre-wrap">{activeDeployLog.log_output || '(no output)'}</pre>
        {/if}
      {/if}
    </div>
  {/if}
</div>
