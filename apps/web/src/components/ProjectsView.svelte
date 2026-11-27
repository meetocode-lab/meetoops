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
  import ExplorerTabs from './ui/ExplorerTabs.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import Skeleton from './ui/Skeleton.svelte';
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
    ArrowLeft,
    ChevronRight,
    Search,
    KeyRound,
    Terminal,
    DownloadCloud,
    Copy,
    Save,
    Server,
    FolderGit2,
    GitCommitHorizontal,
    ShieldCheck,
  } from 'lucide-svelte';

  interface Props {
    lang: Language;
    selectedProjectId?: string | null;
    canOperate: boolean;
    canDelete: boolean;
    onSelectProject?: (id: string | null) => void;
    onOpenTerminal?: () => void;
  }

  let {
    lang,
    selectedProjectId = null,
    canOperate,
    canDelete,
    onSelectProject,
    onOpenTerminal,
  }: Props = $props();

  let projects = $state<ProjectDto[]>([]);
  let sshHosts = $state<SshHostDto[]>([]);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let searchQuery = $state('');
  let busyProjectId = $state<string | null>(null);
  let checkingId = $state<string | null>(null);

  // Modals on List page (never push table down!)
  let showScanModal = $state(false);
  let scanHostId = $state('');
  let scanBaseDir = $state('/home/app');
  let scanning = $state(false);

  let showCreateModal = $state(false);
  let pName = $state('');
  let pSshHostId = $state('');
  let pRepoUrl = $state('');
  let pBranch = $state('main');
  let pDeployPath = $state('/home/app/');
  let pDeploySteps = $state<string[]>(['docker compose up -d --build']);
  let pGitToken = $state('');
  let pEnvContent = $state('');
  let pComposeOverride = $state('');
  let creating = $state(false);

  // Detail Page State (/projects/:id)
  type DetailTab = 'deployments' | 'pipeline' | 'env' | 'settings';
  let detailTab = $state<DetailTab>('deployments');
  let deployments = $state<ProjectDeploymentDto[]>([]);
  let activeDeployLog = $state<ProjectDeploymentDto | null>(null);
  let loadingLogs = $state(false);

  let editBranch = $state('');
  let editSteps = $state<string[]>(['docker compose up -d --build']);
  let editCompose = $state('');
  let editGitToken = $state('');
  let editEnv = $state('');
  let loadingEnv = $state(false);
  let pullingEnv = $state(false);
  let savingConfig = $state(false);

  const commandPresets = [
    { label: 'Docker Compose', cmd: 'docker compose up -d --build' },
    { label: 'Laravel Migrate', cmd: 'docker compose exec -T app php artisan migrate --force' },
    { label: 'Laravel Cache', cmd: 'docker compose exec -T app php artisan optimize' },
    { label: 'Node / Bun Build', cmd: 'bun install && bun run build' },
    { label: 'PM2 Reload', cmd: 'pm2 reload all' },
  ];

  const activeProject = $derived(
    selectedProjectId ? projects.find((p) => p.id === selectedProjectId) ?? null : null,
  );

  const filteredProjects = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.repo_url.toLowerCase().includes(q) ||
        p.deploy_path.toLowerCase().includes(q) ||
        p.branch.toLowerCase().includes(q) ||
        p.ssh_host_name.toLowerCase().includes(q),
    );
  });

  function stepsToCommand(steps: string[]): string {
    return steps
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
      .join('\n');
  }

  function commandToSteps(cmd: string): string[] {
    const lines = cmd
      .split('\n')
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    return lines.length > 0 ? lines : ['docker compose up -d --build'];
  }

  async function loadAll() {
    loading = true;
    errorMsg = null;
    try {
      const [projs, hosts] = await Promise.all([api.listProjects(), api.listSshHosts()]);
      projects = projs;
      sshHosts = hosts;
      if (!pSshHostId && hosts.length > 0) pSshHostId = hosts[0].id;
      if (!scanHostId && hosts.length > 0) scanHostId = hosts[0].id;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load projects.';
    } finally {
      loading = false;
    }
  }

  async function loadProjectDetails(p: ProjectDto) {
    editBranch = p.branch;
    editSteps = commandToSteps(p.deploy_command);
    editCompose = p.compose_override ?? '';
    editGitToken = '';
    editEnv = '';

    loadingLogs = true;
    try {
      deployments = await api.listProjectDeployments(p.id);
      activeDeployLog = deployments[0] ?? null;
    } catch {
      deployments = [];
      activeDeployLog = null;
    } finally {
      loadingLogs = false;
    }

    if (p.has_env_vars) {
      loadingEnv = true;
      try {
        const res = await api.revealProjectEnv(p.id);
        editEnv = res.env_content;
      } catch {
        // user may lack reveal permission
      } finally {
        loadingEnv = false;
      }
    }
  }

  $effect(() => {
    loadAll();
  });

  $effect(() => {
    const p = activeProject;
    if (p) {
      detailTab = 'deployments';
      loadProjectDetails(p);
    }
  });

  function openProject(id: string) {
    onSelectProject?.(id);
  }

  function backToList() {
    onSelectProject?.(null);
  }

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
      showScanModal = false;
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
        deploy_command: stepsToCommand(pDeploySteps) || 'docker compose up -d --build',
        compose_override: pComposeOverride.trim() || undefined,
        git_token: pGitToken.trim() || undefined,
        env_content: pEnvContent.trim() || undefined,
      });
      toast.success('Project created', created.name);
      showCreateModal = false;
      pName = '';
      pRepoUrl = '';
      pBranch = 'main';
      pDeployPath = '/home/app/';
      pDeploySteps = ['docker compose up -d --build'];
      pGitToken = '';
      pEnvContent = '';
      pComposeOverride = '';
      await loadAll();
      openProject(created.id);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create project.';
      errorMsg = msg;
      toast.error('Create failed', msg);
    } finally {
      creating = false;
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
      await loadAll();
      if (activeProject?.id === p.id) {
        detailTab = 'deployments';
        deployments = await api.listProjectDeployments(p.id);
        activeDeployLog = dep;
      } else {
        openProject(p.id);
      }
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
      detailTab = 'deployments';
      deployments = await api.listProjectDeployments(p.id);
      activeDeployLog = dep;
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : `Failed to ${action} project.`;
      errorMsg = msg;
      toast.error('Action failed', msg);
    } finally {
      busyProjectId = null;
    }
  }

  async function handleSavePipeline(e: SubmitEvent) {
    e.preventDefault();
    if (!activeProject) return;
    savingConfig = true;
    errorMsg = null;
    try {
      await api.updateProject(activeProject.id, {
        branch: editBranch.trim(),
        deploy_command: stepsToCommand(editSteps) || 'docker compose up -d --build',
        compose_override: editCompose,
      });
      toast.success('Deploy pipeline saved', activeProject.name);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to save pipeline.';
      errorMsg = msg;
      toast.error('Save failed', msg);
    } finally {
      savingConfig = false;
    }
  }

  async function handlePullEnvFromServer() {
    if (!activeProject) return;
    pullingEnv = true;
    errorMsg = null;
    try {
      const res = await api.pullProjectEnvFromServer(activeProject.id);
      editEnv = res.env_content;
      toast.success('Pulled .env from server', `${activeProject.deploy_path}/.env`);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to read .env from server.';
      errorMsg = msg;
      toast.error('Pull .env failed', msg);
    } finally {
      pullingEnv = false;
    }
  }

  async function handleSaveEnv(e: SubmitEvent) {
    e.preventDefault();
    if (!activeProject) return;
    savingConfig = true;
    errorMsg = null;
    try {
      await api.updateProject(activeProject.id, { env_content: editEnv });
      toast.success('Encrypted .env saved', activeProject.name);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to save .env.';
      errorMsg = msg;
      toast.error('Save failed', msg);
    } finally {
      savingConfig = false;
    }
  }

  async function handleSaveSettings(e: SubmitEvent) {
    e.preventDefault();
    if (!activeProject) return;
    savingConfig = true;
    errorMsg = null;
    try {
      await api.updateProject(activeProject.id, {
        compose_override: editCompose,
        git_token: editGitToken.trim() || undefined,
      });
      editGitToken = '';
      toast.success('Project settings saved', activeProject.name);
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to save settings.';
      errorMsg = msg;
      toast.error('Save failed', msg);
    } finally {
      savingConfig = false;
    }
  }

  async function handleDelete(p: ProjectDto) {
    const ok = await confirmDialog({
      title: 'Delete Project',
      message: 'Remove this project and its encrypted GitHub token / .env from MeetoOps? Files on the server are NOT deleted.',
      targetName: p.name,
      confirmText: 'Delete Project',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteProject(p.id);
      toast.success('Project removed', p.name);
      backToList();
      await loadAll();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to delete project.';
      errorMsg = msg;
      toast.error('Delete failed', msg);
    }
  }

  async function copyLog() {
    if (!activeDeployLog?.log_output) return;
    await navigator.clipboard.writeText(activeDeployLog.log_output);
    toast.info('Deployment log copied');
  }
</script>

<!-- ======================== PROJECT DETAIL PAGE (/projects/:id) ======================== -->
{#if selectedProjectId}
  <div class="w-full space-y-5">
    {#if loading && !activeProject}
      <div class="space-y-3">
        <Skeleton class="h-14 w-full rounded-[var(--radius-lg)]" />
        <Skeleton class="h-80 w-full rounded-[var(--radius-lg)]" />
      </div>
    {:else if !activeProject}
      <div class="p-10 rounded-[var(--radius-lg)] bg-[var(--surface)] border border-[var(--border)] text-center space-y-3">
        <div class="text-sm font-medium text-[var(--foreground)]">Project not found</div>
        <button
          type="button"
          onclick={backToList}
          class="h-[34px] px-3.5 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium"
        >
          Back to Projects
        </button>
      </div>
    {:else}
      <!-- Unified File-Explorer Address + Command Bar -->
      <div class="rounded-[var(--radius-lg)] bg-[var(--surface)] border border-[var(--border)] shadow-[var(--shadow-xs)] overflow-hidden">
        <!-- Top Address & Action Row -->
        <div class="px-4 py-3 flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap items-center gap-2 min-w-0">
            <button
              type="button"
              onclick={backToList}
              class="h-7 px-2 rounded-[var(--radius-md)] hover:bg-[var(--surface-elevated)] text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] inline-flex items-center gap-1 transition-colors"
              title="Back to Projects"
            >
              <ArrowLeft class="w-3.5 h-3.5" />
              <span>{lang === 'id' ? 'Proyek' : 'Projects'}</span>
            </button>
            <ChevronRight class="w-3.5 h-3.5 text-[var(--subtle-foreground)] shrink-0" />
            <h1 class="text-base font-semibold text-[var(--foreground)] tracking-tight truncate">
              {activeProject.name}
            </h1>
            <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-[var(--radius-sm)] bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[11px] font-medium">
              <GitBranch class="w-3 h-3" />
              {activeProject.branch}
            </span>
            <span
              class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium capitalize {activeProject.last_status === 'running'
                ? 'bg-[var(--success-subtle)] text-[var(--success)]'
                : activeProject.last_status === 'failed'
                  ? 'bg-[var(--danger-subtle)] text-[var(--danger)]'
                  : activeProject.last_status === 'deploying'
                    ? 'bg-[var(--info-subtle)] text-[var(--info)]'
                    : 'bg-[var(--surface-secondary)] text-[var(--muted-foreground)]'}"
            >
              <span
                class="w-1.5 h-1.5 rounded-full {activeProject.last_status === 'running'
                  ? 'bg-[var(--success)]'
                  : activeProject.last_status === 'failed'
                    ? 'bg-[var(--danger)]'
                    : activeProject.last_status === 'deploying'
                      ? 'bg-[var(--info)] animate-pulse'
                      : 'bg-[var(--subtle-foreground)]'}"
              ></span>
              {activeProject.last_status}
            </span>
            {#if activeProject.update_available}
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-[var(--warning-subtle)] text-[var(--warning)] text-[11px] font-medium">
                <ArrowUpCircle class="w-3 h-3" /> Update ({activeProject.remote_head_commit})
              </span>
            {/if}
          </div>

          <!-- Precision Command Strip -->
          <div class="flex flex-wrap items-center gap-1.5 shrink-0">
            <button
              type="button"
              disabled={checkingId === activeProject.id}
              onclick={() => handleCheckRemote(activeProject)}
              class="h-[32px] px-2.5 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-xs font-medium text-[var(--foreground)] inline-flex items-center gap-1.5 transition-colors disabled:opacity-40"
            >
              <RefreshCw class="w-3.5 h-3.5 text-[var(--muted-foreground)] {checkingId === activeProject.id ? 'animate-spin' : ''}" />
              Check GitHub
            </button>
            {#if canOperate}
              <button
                type="button"
                disabled={busyProjectId === activeProject.id}
                onclick={() => handleDeploy(activeProject)}
                class="h-[32px] px-3.5 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 shadow-[var(--shadow-xs)] transition-colors disabled:opacity-40"
              >
                {#if busyProjectId === activeProject.id}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Rocket class="w-3.5 h-3.5" />{/if}
                Deploy Now
              </button>
              <div class="inline-flex items-center rounded-[var(--radius-md)] border border-[var(--border-strong)] overflow-hidden divide-x divide-[var(--border)]">
                <button
                  type="button"
                  disabled={busyProjectId === activeProject.id}
                  onclick={() => handleAction(activeProject, 'restart')}
                  class="h-[30px] px-2.5 bg-[var(--surface)] hover:bg-[var(--surface-elevated)] text-xs font-medium text-[var(--foreground)] inline-flex items-center gap-1 transition-colors disabled:opacity-40"
                  title="Restart project"
                >
                  <RotateCw class="w-3.5 h-3.5 text-[var(--primary)]" /> Restart
                </button>
                <button
                  type="button"
                  disabled={busyProjectId === activeProject.id}
                  onclick={() => handleAction(activeProject, 'stop')}
                  class="h-[30px] px-2.5 bg-[var(--surface)] hover:bg-[var(--danger-subtle)] text-xs font-medium text-[var(--danger)] inline-flex items-center gap-1 transition-colors disabled:opacity-40"
                  title="Stop project"
                >
                  <Square class="w-3 h-3" /> Stop
                </button>
              </div>
            {/if}
          </div>
        </div>

        <!-- Compact Path & Repo Metadata Strip -->
        <div class="px-4 py-2 bg-[var(--surface-secondary)]/55 border-t border-[var(--border)] flex flex-wrap items-center gap-x-4 gap-y-1 text-[11.5px] text-[var(--muted-foreground)]">
          <span class="inline-flex items-center gap-1.5 font-medium text-[var(--foreground)]">
            <Server class="w-3.5 h-3.5 text-[var(--primary)]" />
            {activeProject.ssh_host_name}
          </span>
          <span class="text-[var(--border-strong)]" aria-hidden="true">|</span>
          <span class="inline-flex items-center gap-1.5 font-mono truncate max-w-md" title={activeProject.repo_url}>
            <FolderGit2 class="w-3.5 h-3.5 text-[var(--subtle-foreground)] shrink-0" />
            {activeProject.repo_url}
          </span>
          <span class="text-[var(--border-strong)]" aria-hidden="true">|</span>
          <span class="font-mono text-[var(--foreground)]/85">{activeProject.deploy_path}</span>
          {#if activeProject.last_deployed_commit}
            <span class="text-[var(--border-strong)]" aria-hidden="true">|</span>
            <span class="inline-flex items-center gap-1 font-mono tabular-nums">
              <GitCommitHorizontal class="w-3.5 h-3.5 text-[var(--subtle-foreground)]" />
              {activeProject.last_deployed_commit}
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

      <!-- File-Explorer Connected Tab Workbench -->
      <ExplorerTabs
        bind:active={detailTab}
        tabs={[
          {
            id: 'deployments',
            label: 'Deployments & Logs',
            icon: ScrollText,
            badge: deployments.length > 0 ? deployments.length : undefined,
          },
          {
            id: 'pipeline',
            label: 'Deploy Pipeline',
            icon: Terminal,
            badge: `${editSteps.filter((s) => s.trim()).length || 1} step${(editSteps.filter((s) => s.trim()).length || 1) > 1 ? 's' : ''}`,
          },
          {
            id: 'env',
            label: 'Environment (.env)',
            icon: KeyRound,
            dot: activeProject.has_env_vars ? 'success' : undefined,
          },
          {
            id: 'settings',
            label: 'Settings',
            icon: Settings2,
          },
        ]}
      >
        {#snippet actions()}
          {#if detailTab === 'deployments' && activeDeployLog}
            <button
              type="button"
              onclick={copyLog}
              class="h-7 px-2.5 rounded-[var(--radius-sm)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] font-medium text-[var(--foreground)] inline-flex items-center gap-1 transition-colors"
            >
              <Copy class="w-3 h-3" /> Copy Log
            </button>
          {:else if detailTab === 'env'}
            <button
              type="button"
              disabled={pullingEnv}
              onclick={handlePullEnvFromServer}
              class="h-7 px-2.5 rounded-[var(--radius-sm)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-[11px] font-medium text-[var(--foreground)] inline-flex items-center gap-1.5 transition-colors disabled:opacity-50"
            >
              {#if pullingEnv}<Loader2 class="w-3 h-3 animate-spin" />{:else}<DownloadCloud class="w-3 h-3 text-[var(--primary)]" />{/if}
              Pull .env from Server
            </button>
          {/if}
        {/snippet}

        <!-- Tab 1: Deployments & Logs -->
        {#if detailTab === 'deployments'}
          {#if loadingLogs}
            <div class="p-5">
              <TableSkeleton rows={4} columns={3} />
            </div>
          {:else if deployments.length === 0}
            <!-- Purposeful Readiness Workbench (no barren empty void) -->
            <div class="grid grid-cols-1 lg:grid-cols-12 divide-y lg:divide-y-0 lg:divide-x divide-[var(--border)]">
              <div class="lg:col-span-5 p-6 flex flex-col justify-between gap-6">
                <div class="space-y-3">
                  <div class="inline-flex items-center gap-2 text-xs font-semibold text-[var(--primary)]">
                    <Rocket class="w-4 h-4" />
                    <span>Ready for First Deployment</span>
                  </div>
                  <h2 class="text-base font-semibold text-[var(--foreground)] tracking-tight">
                    No deployment runs recorded yet
                  </h2>
                  <p class="text-xs text-[var(--muted-foreground)] leading-relaxed">
                    MeetoOps will connect to <strong class="text-[var(--foreground)]">{activeProject.ssh_host_name}</strong> via SSH, sync branch <code class="font-mono text-[var(--foreground)]">{activeProject.branch}</code>, write your encrypted <code class="font-mono text-[var(--foreground)]">.env</code>, and execute your configured post-pull pipeline with full live output capture.
                  </p>
                </div>

                <div class="flex flex-wrap items-center gap-2 pt-2">
                  {#if canOperate}
                    <button
                      type="button"
                      disabled={busyProjectId === activeProject.id}
                      onclick={() => handleDeploy(activeProject)}
                      class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 transition-colors disabled:opacity-40"
                    >
                      {#if busyProjectId === activeProject.id}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Rocket class="w-3.5 h-3.5" />{/if}
                      Run First Deployment
                    </button>
                  {/if}
                  <button
                    type="button"
                    onclick={() => (detailTab = 'pipeline')}
                    class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-xs font-medium text-[var(--foreground)] inline-flex items-center gap-1.5 transition-colors"
                  >
                    <Terminal class="w-3.5 h-3.5 text-[var(--primary)]" /> Edit Pipeline
                  </button>
                </div>
              </div>

              <!-- Right: Live Execution Plan Preview -->
              <div class="lg:col-span-7 bg-[#071426] text-[#cbd5e1] p-5 flex flex-col justify-between font-mono text-xs">
                <div class="space-y-3">
                  <div class="flex items-center justify-between text-[11px] text-[#94a3b8] pb-2 border-b border-[#1e3552]">
                    <span>EXECUTION PLAN · {activeProject.ssh_host_name}</span>
                    <span>{activeProject.deploy_path}</span>
                  </div>
                  <div class="space-y-2 leading-relaxed">
                    <div class="flex items-start gap-2.5">
                      <span class="text-[#38bdf8] select-none">01</span>
                      <span>cd {activeProject.deploy_path} && git fetch origin && git pull origin {activeProject.branch}</span>
                    </div>
                    {#if activeProject.has_env_vars}
                      <div class="flex items-start gap-2.5">
                        <span class="text-[#4ade80] select-none">02</span>
                        <span>sync encrypted .env → chmod 600 .env</span>
                      </div>
                    {/if}
                    {#each (editSteps.filter((s) => s.trim()).length ? editSteps.filter((s) => s.trim()) : ['docker compose up -d --build']) as step, idx}
                      <div class="flex items-start gap-2.5">
                        <span class="text-[#60a5fa] select-none">{String(idx + (activeProject.has_env_vars ? 3 : 2)).padStart(2, '0')}</span>
                        <span class="text-[#f8fafc]">{step}</span>
                      </div>
                    {/each}
                  </div>
                </div>
                <div class="mt-6 pt-3 border-t border-[#1e3552] text-[11px] text-[#64748b] flex items-center justify-between">
                  <span>All stdout/stderr will stream & archive here</span>
                  <span>SSH · non-interactive</span>
                </div>
              </div>
            </div>
          {:else}
            <div class="grid grid-cols-1 lg:grid-cols-12 divide-y lg:divide-y-0 lg:divide-x divide-[var(--border)]">
              <div class="lg:col-span-4 divide-y divide-[var(--border-subtle)] max-h-[60vh] overflow-y-auto">
                {#each deployments as d}
                  <button
                    type="button"
                    onclick={() => (activeDeployLog = d)}
                    class="w-full text-left px-4 py-3 transition-colors {activeDeployLog?.id === d.id
                      ? 'bg-[var(--primary-subtle)]'
                      : 'hover:bg-[var(--surface-elevated)]/60'}"
                  >
                    <div class="flex items-center justify-between gap-2">
                      <span class="text-xs font-semibold capitalize text-[var(--foreground)]">{d.action}</span>
                      <span
                        class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase {d.status === 'success'
                          ? 'bg-[var(--success-subtle)] text-[var(--success)]'
                          : d.status === 'failed'
                            ? 'bg-[var(--danger-subtle)] text-[var(--danger)]'
                            : 'bg-[var(--info-subtle)] text-[var(--info)]'}"
                      >
                        {d.status}
                      </span>
                    </div>
                    <div class="mt-1 flex items-center justify-between text-[11px] text-[var(--muted-foreground)] font-mono tabular-nums">
                      <span>{d.commit_hash ?? '—'}</span>
                      <span>{new Date(d.started_at).toLocaleString()}</span>
                    </div>
                  </button>
                {/each}
              </div>

              <div class="lg:col-span-8 flex flex-col overflow-hidden bg-[#071426]">
                <div class="px-4 py-2 border-b border-[#1e3552] flex items-center justify-between text-[11px] font-mono text-[#94a3b8]">
                  <span>
                    {activeDeployLog
                      ? `${activeDeployLog.action} · ${activeDeployLog.status} · ${activeDeployLog.commit_hash ?? '—'}`
                      : 'Output Log'}
                  </span>
                  <span>{activeDeployLog ? new Date(activeDeployLog.started_at).toLocaleTimeString() : ''}</span>
                </div>
                <pre class="p-4 text-[#e2e8f0] font-mono text-xs overflow-auto h-[55vh] whitespace-pre-wrap leading-relaxed">{activeDeployLog?.log_output || '(no output)'}</pre>
              </div>
            </div>
          {/if}

        <!-- Tab 2: Deploy Pipeline -->
        {:else if detailTab === 'pipeline'}
          <form onsubmit={handleSavePipeline} class="p-6 space-y-6">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-5 pb-5 border-b border-[var(--border)]">
              <div class="space-y-1">
                <h3 class="text-xs font-semibold text-[var(--foreground)]">Source Branch</h3>
                <p class="text-[11px] text-[var(--muted-foreground)]">
                  Target branch fetched & pulled before running commands.
                </p>
              </div>
              <div class="md:col-span-2 max-w-xs">
                <input
                  type="text"
                  required
                  bind:value={editBranch}
                  class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)]"
                />
              </div>
            </div>

            <div class="space-y-4">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <h3 class="text-xs font-semibold text-[var(--foreground)]">Post-Pull Deploy Commands (Sequential)</h3>
                  <p class="text-[11px] text-[var(--muted-foreground)] mt-0.5">
                    Executed in order inside <code class="font-mono text-[var(--foreground)]">{activeProject.deploy_path}</code> right after <code class="font-mono">git pull</code> & <code class="font-mono">.env</code> sync.
                  </p>
                </div>
                <button
                  type="button"
                  onclick={() => (editSteps = [...editSteps, ''])}
                  class="h-[30px] px-2.5 rounded-[var(--radius-md)] bg-[var(--primary-subtle)] hover:bg-[var(--primary)] hover:text-white text-[var(--primary)] text-[11px] font-medium inline-flex items-center gap-1 transition-colors"
                >
                  <Plus class="w-3.5 h-3.5" /> Add Step
                </button>
              </div>

              <div class="space-y-2.5">
                {#each editSteps as _, idx}
                  <div class="flex items-center gap-2.5">
                    <span class="w-7 h-7 rounded-full bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[11px] font-semibold inline-flex items-center justify-center shrink-0 tabular-nums">
                      {idx + 1}
                    </span>
                    <input
                      type="text"
                      bind:value={editSteps[idx]}
                      placeholder="e.g. docker compose up -d --build"
                      class="flex-1 px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)]"
                    />
                    {#if editSteps.length > 1}
                      <button
                        type="button"
                        onclick={() => (editSteps = editSteps.filter((_, i) => i !== idx))}
                        class="h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] border border-[var(--border)] text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-colors"
                        title="Remove step"
                      >
                        <X class="w-3.5 h-3.5" />
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>

              <div class="flex flex-wrap items-center gap-1.5 pt-1">
                <span class="text-[11px] font-medium text-[var(--muted-foreground)] mr-1">Presets:</span>
                {#each commandPresets as preset}
                  <button
                    type="button"
                    onclick={() => (editSteps = [...editSteps.filter((s) => s.trim()), preset.cmd])}
                    class="px-2.5 py-1 rounded-full bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] font-medium text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
                  >
                    + {preset.label}
                  </button>
                {/each}
              </div>
            </div>

            <div class="flex justify-end pt-4 border-t border-[var(--border)]">
              <button
                type="submit"
                disabled={!canOperate || savingConfig}
                class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 transition-colors disabled:opacity-50"
              >
                {#if savingConfig}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Save class="w-3.5 h-3.5" />{/if}
                Save Pipeline
              </button>
            </div>
          </form>

        <!-- Tab 3: Environment (.env) -->
        {:else if detailTab === 'env'}
          <form onsubmit={handleSaveEnv} class="p-6 space-y-4">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div class="flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
                <ShieldCheck class="w-4 h-4 text-[var(--success)] shrink-0" />
                <span>
                  Encrypted with <strong class="text-[var(--foreground)]">AES-256-GCM</strong> in MeetoOps Vault and synced to <code class="font-mono text-[var(--foreground)]">{activeProject.deploy_path}/.env</code> (<code class="font-mono">chmod 600</code>).
                </span>
              </div>
            </div>

            {#if loadingEnv}
              <Skeleton class="h-56 w-full rounded-[var(--radius-md)]" />
            {:else}
              <textarea
                rows="13"
                bind:value={editEnv}
                placeholder="APP_ENV=production&#10;DB_HOST=127.0.0.1"
                class="w-full p-4 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)] leading-relaxed"
              ></textarea>
            {/if}

            <div class="flex justify-end pt-2 border-t border-[var(--border)]">
              <button
                type="submit"
                disabled={!canOperate || savingConfig}
                class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 transition-colors disabled:opacity-50"
              >
                {#if savingConfig}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Save class="w-3.5 h-3.5" />{/if}
                Save Encrypted .env
              </button>
            </div>
          </form>

        <!-- Tab 4: Settings & Danger Zone -->
        {:else}
          <div class="divide-y divide-[var(--border)]">
            <form onsubmit={handleSaveSettings} class="p-6 space-y-5">
              <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
                <div class="space-y-1">
                  <h3 class="text-xs font-semibold text-[var(--foreground)]">GitHub Personal Access Token</h3>
                  <p class="text-[11px] text-[var(--muted-foreground)]">
                    Optional for private HTTPS repos. Leave blank to keep existing token or SSH key auth.
                  </p>
                </div>
                <div class="md:col-span-2 max-w-md">
                  <input
                    type="password"
                    bind:value={editGitToken}
                    placeholder="ghp_xxxxxxxx (leave blank to keep existing)"
                    class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)]"
                  />
                </div>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-3 gap-5 pt-5 border-t border-[var(--border)]">
                <div class="space-y-1">
                  <h3 class="text-xs font-semibold text-[var(--foreground)]">Custom docker-compose.yml</h3>
                  <p class="text-[11px] text-[var(--muted-foreground)]">
                    Override the repository's compose file during deploy. Leave blank to use the repo's own file.
                  </p>
                </div>
                <div class="md:col-span-2">
                  <textarea
                    rows="8"
                    bind:value={editCompose}
                    class="w-full p-3.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs text-[var(--foreground)] leading-relaxed"
                  ></textarea>
                </div>
              </div>

              <div class="flex justify-end pt-3 border-t border-[var(--border)]">
                <button
                  type="submit"
                  disabled={!canOperate || savingConfig}
                  class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 transition-colors disabled:opacity-50"
                >
                  {#if savingConfig}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<Save class="w-3.5 h-3.5" />{/if}
                  Save Settings
                </button>
              </div>
            </form>

            {#if canDelete}
              <div class="px-6 py-4 bg-[var(--danger-subtle)]/35 flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h3 class="text-xs font-semibold text-[var(--danger)]">Danger Zone</h3>
                  <p class="text-[11px] text-[var(--muted-foreground)] mt-0.5">
                    Remove <strong class="text-[var(--foreground)]">{activeProject.name}</strong> and its stored Vault secrets from MeetoOps (server files remain untouched).
                  </p>
                </div>
                <button
                  type="button"
                  onclick={() => handleDelete(activeProject)}
                  class="h-[32px] px-3.5 rounded-[var(--radius-md)] bg-[var(--danger)] text-white text-xs font-medium inline-flex items-center gap-1.5 hover:opacity-90 transition-opacity"
                >
                  <Trash2 class="w-3.5 h-3.5" /> Delete Project
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </ExplorerTabs>
    {/if}
  </div>

<!-- ======================== PROJECTS LIST PAGE (/projects) ======================== -->
{:else}
  <div class="w-full space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
          <GitBranch class="w-5 h-5 text-[var(--primary)]" />
          {lang === 'id' ? 'Proyek GitHub & Deploy' : 'GitHub Projects & Deploy'}
        </h1>
        <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
          {lang === 'id'
            ? 'Klik nama project untuk membuka halaman detail (Logs, Pipeline, .env, & Settings).'
            : 'Click any project to open its dedicated workspace for logs, deploy pipeline, .env, and settings.'}
        </p>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <div class="relative">
          <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder={lang === 'id' ? 'Cari project…' : 'Search projects…'}
            bind:value={searchQuery}
            class="pl-8 pr-3 w-48 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
          />
        </div>
        {#if canOperate}
          <button
            type="button"
            onclick={() => (showScanModal = true)}
            class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border-strong)] text-xs font-medium inline-flex items-center gap-1.5"
          >
            <FolderSearch class="w-3.5 h-3.5 text-[var(--primary)]" />
            {lang === 'id' ? 'Scan Folder Existing' : 'Scan Existing Repos'}
          </button>
          <button
            type="button"
            onclick={() => (showCreateModal = true)}
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
          <strong>No SSH host configured yet.</strong> Add your VPS SSH connection first in <strong>Terminal → Host SSH</strong>.
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

    <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] overflow-x-auto">
      {#if loading && projects.length === 0}
        <TableSkeleton rows={5} columns={5} />
      {:else if filteredProjects.length === 0}
        <div class="p-10 text-center space-y-2">
          <GitBranch class="w-6 h-6 text-[var(--muted-foreground)] mx-auto" />
          <div class="text-xs text-[var(--muted-foreground)]">
            {searchQuery.trim()
              ? 'No projects match your search.'
              : 'No GitHub projects registered yet. Click Scan Existing Repos or New Project to get started.'}
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
            {#each filteredProjects as p}
              <tr class="hover:bg-[var(--surface-elevated)] transition-colors">
                <td class="px-4 py-3">
                  <button
                    type="button"
                    onclick={() => openProject(p.id)}
                    class="text-left group block"
                  >
                    <div class="font-semibold text-[var(--foreground)] group-hover:text-[var(--primary)] transition-colors flex items-center gap-2">
                      <span>{p.name}</span>
                      <span class="px-1.5 py-0.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[10px]">
                        {p.branch}
                      </span>
                      {#if p.has_env_vars}
                        <span class="px-1.5 py-0.5 rounded bg-[var(--surface-secondary)] text-[var(--muted-foreground)] font-mono text-[10px]">.env</span>
                      {/if}
                    </div>
                    <div class="font-mono text-[11px] text-[var(--muted-foreground)] truncate max-w-[340px] mt-0.5">{p.repo_url}</div>
                    <div class="font-mono text-[10px] text-[var(--muted-foreground)]">{p.deploy_path}</div>
                  </button>
                </td>
                <td class="px-4 py-3 text-[var(--foreground)]">{p.ssh_host_name}</td>
                <td class="px-4 py-3 font-mono text-[11px]">
                  <div class="text-[var(--foreground)]">{p.last_deployed_commit ?? '—'}</div>
                  {#if p.update_available}
                    <div class="inline-flex items-center gap-1 mt-1 px-1.5 py-0.5 rounded bg-[var(--warning-subtle)] text-[var(--warning)] text-[10px] font-sans font-medium">
                      <ArrowUpCircle class="w-3 h-3" /> Update: {p.remote_head_commit}
                    </div>
                  {:else if p.remote_head_commit}
                    <div class="inline-flex items-center gap-1 mt-1 text-[var(--success)] text-[10px] font-sans">
                      <CheckCircle2 class="w-3 h-3" /> Synced
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
                  <div class="inline-flex items-center gap-2">
                    {#if canOperate}
                      <button
                        type="button"
                        disabled={busyProjectId === p.id}
                        onclick={() => handleDeploy(p)}
                        class="h-[30px] px-2.5 rounded-[var(--radius-md)] bg-[var(--primary)] hover:bg-[var(--primary-hover)] text-[var(--primary-foreground)] text-[11px] font-medium inline-flex items-center gap-1 disabled:opacity-40"
                      >
                        {#if busyProjectId === p.id}<Loader2 class="w-3 h-3 animate-spin" />{:else}<Rocket class="w-3 h-3" />{/if}
                        Deploy
                      </button>
                    {/if}
                    <button
                      type="button"
                      onclick={() => openProject(p.id)}
                      class="h-[30px] px-2.5 rounded-[var(--radius-md)] border border-[var(--border-strong)] bg-[var(--surface)] hover:bg-[var(--surface-elevated)] text-[11px] font-medium text-[var(--foreground)] inline-flex items-center gap-1"
                    >
                      Manage <ChevronRight class="w-3 h-3" />
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
{/if}

<!-- ======================== MODAL: SCAN EXISTING REPOS ======================== -->
{#if showScanModal}
  <div
    class="fixed inset-0 z-[105] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
  >
    <form
      onsubmit={handleScanExisting}
      class="w-full max-w-lg rounded-2xl border border-[var(--border-strong)] bg-[var(--surface)] p-6 shadow-2xl space-y-4"
    >
      <div class="flex items-start justify-between gap-3">
        <div>
          <h2 class="text-base font-semibold text-[var(--foreground)] flex items-center gap-2">
            <FolderSearch class="w-4 h-4 text-[var(--primary)]" />
            {lang === 'id' ? 'Scan Folder GitHub Existing' : 'Scan Existing GitHub Folders'}
          </h2>
          <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
            Auto-discovers <code class="font-mono">.git</code> repositories (origin URL, branch, HEAD commit, and <code class="font-mono">.env</code>) up to 3 levels deep.
          </p>
        </div>
        <button type="button" onclick={() => (showScanModal = false)} class="p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="space-y-3">
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Target SSH Host</span>
          <Select
            class="w-full"
            bind:value={scanHostId}
            placeholder="Select SSH host…"
            options={sshHosts.map((h) => ({ value: h.id, label: h.name, description: `${h.username}@${h.host}:${h.port}` }))}
          />
        </div>
        <label class="block space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Base Directory on Server</span>
          <input
            type="text"
            required
            bind:value={scanBaseDir}
            placeholder="/var/www/html or /home/app"
            class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"
          />
        </label>
      </div>

      <div class="flex justify-end gap-2 pt-2">
        <button type="button" onclick={() => (showScanModal = false)} class="h-[34px] px-3.5 rounded-[var(--radius-md)] border border-[var(--border-strong)] text-xs font-medium">
          Cancel
        </button>
        <button
          type="submit"
          disabled={scanning || !scanHostId}
          class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50"
        >
          {#if scanning}<Loader2 class="w-3.5 h-3.5 animate-spin" />{:else}<FolderSearch class="w-3.5 h-3.5" />{/if}
          {scanning ? 'Scanning…' : 'Scan & Import'}
        </button>
      </div>
    </form>
  </div>
{/if}

<!-- ======================== MODAL: CREATE NEW PROJECT ======================== -->
{#if showCreateModal}
  <div
    class="fixed inset-0 z-[105] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
  >
    <form
      onsubmit={handleCreate}
      class="w-full max-w-3xl max-h-[90vh] overflow-y-auto rounded-2xl border border-[var(--border-strong)] bg-[var(--surface)] p-6 shadow-2xl space-y-4"
    >
      <div class="flex items-start justify-between gap-3">
        <div>
          <h2 class="text-base font-semibold text-[var(--foreground)]">Register New GitHub Project</h2>
          <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
            Clone a repository via SSH and configure its post-pull deployment pipeline & encrypted .env.
          </p>
        </div>
        <button type="button" onclick={() => (showCreateModal = false)} class="p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
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
          <span class="text-[11px] text-[var(--muted-foreground)]">GitHub Repository URL</span>
          <input type="text" required placeholder="https://github.com/org/repo.git or git@github.com:org/repo.git" bind:value={pRepoUrl} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">GitHub PAT (optional)</span>
          <input type="password" placeholder="ghp_xxxxxxxx" bind:value={pGitToken} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <label class="md:col-span-3 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Deploy Directory on Server</span>
          <input type="text" required placeholder="/home/app/my-saas-api" bind:value={pDeployPath} class="w-full px-3 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
      </div>

      <div class="space-y-2 p-3.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border)]">
        <div class="flex items-center justify-between gap-2">
          <span class="text-xs font-semibold text-[var(--foreground)]">Post-Pull Deploy Commands</span>
          <button
            type="button"
            onclick={() => (pDeploySteps = [...pDeploySteps, ''])}
            class="h-[28px] px-2.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] text-[11px] font-medium inline-flex items-center gap-1"
          >
            <Plus class="w-3 h-3" /> Add Step
          </button>
        </div>
        <div class="space-y-2">
          {#each pDeploySteps as _, idx}
            <div class="flex items-center gap-2">
              <span class="w-12 shrink-0 text-[11px] font-mono text-[var(--muted-foreground)]">Step {idx + 1}</span>
              <input
                type="text"
                bind:value={pDeploySteps[idx]}
                placeholder="docker compose up -d --build"
                class="flex-1 px-3 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] font-mono text-xs"
              />
              {#if pDeploySteps.length > 1}
                <button
                  type="button"
                  onclick={() => (pDeploySteps = pDeploySteps.filter((_, i) => i !== idx))}
                  class="h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] border border-[var(--border)] text-[var(--danger)]"
                >
                  <X class="w-3.5 h-3.5" />
                </button>
              {/if}
            </div>
          {/each}
        </div>
        <div class="flex flex-wrap items-center gap-1.5 pt-1">
          {#each commandPresets as preset}
            <button
              type="button"
              onclick={() => (pDeploySteps = [...pDeploySteps.filter((s) => s.trim()), preset.cmd])}
              class="px-2 py-0.5 rounded bg-[var(--surface)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] text-[var(--muted-foreground)]"
            >
              + {preset.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">.env Content (encrypted, optional)</span>
          <textarea rows="4" bind:value={pEnvContent} placeholder="NODE_ENV=production" class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">docker-compose.yml Override (optional)</span>
          <textarea rows="4" bind:value={pComposeOverride} placeholder="Leave blank to use repo compose file" class="w-full p-2.5 rounded-[var(--radius-md)] bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs"></textarea>
        </label>
      </div>

      <div class="flex justify-end gap-2 pt-2">
        <button type="button" onclick={() => (showCreateModal = false)} class="h-[34px] px-3.5 rounded-[var(--radius-md)] border border-[var(--border-strong)] text-xs font-medium">
          Cancel
        </button>
        <button type="submit" disabled={creating} class="h-[34px] px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center gap-1.5 disabled:opacity-50">
          {#if creating}<Loader2 class="w-3.5 h-3.5 animate-spin" />{/if}
          Create Project
        </button>
      </div>
    </form>
  </div>
{/if}
