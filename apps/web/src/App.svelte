<script lang="ts">
  import {
    api,
    setCsrfToken,
    type AuthSessionDto,
    type ServerSummary,
    type SetupStatusDto,
  } from './lib/api';
  import { translations, type Language } from './lib/i18n';
  import {
    applyThemePreference,
    loadThemePreference,
    watchSystemTheme,
    type ThemePreference,
  } from './lib/theme';
  import ThemeToggle from './components/ThemeToggle.svelte';
  import Skeleton from './components/ui/Skeleton.svelte';
  import Select from './components/ui/Select.svelte';
  import ToastContainer from './components/ui/ToastContainer.svelte';
  import ConfirmModal from './components/ui/ConfirmModal.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import SetupWizard from './components/SetupWizard.svelte';
  import LoginView from './components/LoginView.svelte';
  import DashboardView from './components/DashboardView.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import ContainersView from './components/ContainersView.svelte';
  import ImagesView from './components/ImagesView.svelte';
  import VolumesView from './components/VolumesView.svelte';
  import NetworksView from './components/NetworksView.svelte';
  import LogCenterView from './components/LogCenterView.svelte';
  import MonitoringView from './components/MonitoringView.svelte';
  import DiscoveryView from './components/DiscoveryView.svelte';
  import DatabaseExplorerView from './components/DatabaseExplorerView.svelte';
  import RedisExplorerView from './components/RedisExplorerView.svelte';
  import TerminalView from './components/TerminalView.svelte';
  import StacksView from './components/StacksView.svelte';
  import ServersView from './components/ServersView.svelte';
  import AlertsView from './components/AlertsView.svelte';
  import {
    LayoutDashboard,
    Activity,
    Bell,
    Box,
    Layers,
    HardDrive,
    Network,
    FileCode2,
    Radar,
    Database,
    KeyRound,
    ScrollText,
    Terminal,
    Server,
    Settings,
    LogOut,
    PanelLeftClose,
    PanelLeftOpen,
    Globe,
    Search,
  } from 'lucide-svelte';

  type PageKey =
    | 'dashboard'
    | 'monitoring'
    | 'alerts'
    | 'containers'
    | 'images'
    | 'volumes'
    | 'networks'
    | 'stacks'
    | 'discovery'
    | 'databases'
    | 'redis'
    | 'logs'
    | 'terminal'
    | 'servers'
    | 'settings';

  const validPages: PageKey[] = [
    'dashboard',
    'monitoring',
    'alerts',
    'containers',
    'images',
    'volumes',
    'networks',
    'stacks',
    'discovery',
    'databases',
    'redis',
    'logs',
    'terminal',
    'servers',
    'settings',
  ];

  function pageFromLocation(): PageKey {
    const seg = window.location.pathname.replace(/^\/+|\/+$/g, '').split('/')[0] ?? '';
    return (validPages as string[]).includes(seg) ? (seg as PageKey) : 'dashboard';
  }

  let bootLoading = $state(true);
  let bootError = $state<string | null>(null);
  let setupStatus = $state<SetupStatusDto | null>(null);
  let session = $state<AuthSessionDto | null>(null);
  let lang = $state<Language>('en');
  let theme = $state<ThemePreference>(loadThemePreference());
  let appName = $state('MeetoOps');
  let activePage = $state<PageKey>(pageFromLocation());

  function navigate(page: PageKey, replace = false) {
    activePage = page;
    const targetPath = `/${page}`;
    if (window.location.pathname !== targetPath) {
      if (replace) {
        window.history.replaceState({ page }, '', targetPath);
      } else {
        window.history.pushState({ page }, '', targetPath);
      }
    }
  }

  $effect(() => {
    const onPopState = () => {
      activePage = pageFromLocation();
    };
    window.addEventListener('popstate', onPopState);
    if (window.location.pathname === '/' || window.location.pathname === '') {
      window.history.replaceState({ page: activePage }, '', `/${activePage}`);
    }
    return () => window.removeEventListener('popstate', onPopState);
  });
  let selectedLogContainerId = $state<string | null>(null);
  let selectedTerminalContainerId = $state<string | null>(null);
  let selectedServiceId = $state<string | null>(null);
  let sidebarCollapsed = $state(false);
  let commandPaletteOpen = $state(false);
  let firingAlertsCount = $state(0);
  let serverList = $state<ServerSummary[]>([]);
  let activeServerId = $state<string>(localStorage.getItem('meetoops_active_server') || 'local');

  const pageTitles: Record<PageKey, string> = {
    dashboard: 'Dashboard',
    monitoring: 'Monitoring',
    alerts: 'Alerts',
    containers: 'Containers',
    images: 'Images',
    volumes: 'Volumes',
    networks: 'Networks',
    stacks: 'Compose Stacks',
    discovery: 'Service Discovery',
    databases: 'Database Explorer',
    redis: 'Redis Explorer',
    logs: 'Log Center',
    terminal: 'Terminal',
    servers: 'Servers',
    settings: 'Settings',
  };

  $effect(() => {
    document.title = `${pageTitles[activePage]} · ${appName}`;
  });

  $effect(() => {
    localStorage.setItem('meetoops_active_server', activeServerId);
  });

  const serverOptions = $derived([
    { value: 'local', label: 'Local Host' },
    ...serverList
      .filter((s) => s.id !== 'local')
      .map((s) => ({
        value: s.id,
        label: `${s.name} (${s.status})`,
      })),
  ]);

  function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      if (session) {
        commandPaletteOpen = !commandPaletteOpen;
      }
    }
  }

  $effect(() => {
    if (!session) {
      firingAlertsCount = 0;
      serverList = [];
      return;
    }
    let active = true;
    async function pollShell() {
      try {
        const [summary, srvs] = await Promise.all([
          api.alertsSummary().catch(() => ({ firing: 0, acknowledged: 0 })),
          api.listServers().catch(() => [] as ServerSummary[]),
        ]);
        if (!active) return;
        firingAlertsCount = summary.firing;
        serverList = srvs;
      } catch {
        // silent shell poll
      }
    }
    pollShell();
    const timer = setInterval(pollShell, 15000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  });

  const t = $derived(translations[lang]);

  $effect(() => {
    applyThemePreference(theme);
  });

  $effect(() => {
    return watchSystemTheme(() => theme);
  });

  function changeTheme(next: ThemePreference) {
    theme = next;
    applyThemePreference(next);
  }

  async function bootstrap() {
    bootLoading = true;
    bootError = null;
    try {
      const status = await api.getSetupStatus();
      setupStatus = status;
      appName = status.app_name || 'MeetoOps';
      const savedLang = localStorage.getItem('meetoops_lang');
      if (savedLang === 'en' || savedLang === 'id') {
        lang = savedLang;
      } else if (status.language === 'en' || status.language === 'id') {
        lang = status.language;
      }

      if (status.initialized) {
        try {
          const me = await api.me();
          setCsrfToken(me.csrf_token);
          session = me;
        } catch {
          session = null;
        }
      }
    } catch (err) {
      bootError = err instanceof Error ? err.message : 'Unable to reach MeetoOps Server.';
    } finally {
      bootLoading = false;
    }
  }

  $effect(() => {
    bootstrap();
  });

  function changeLang(next: Language) {
    lang = next;
    localStorage.setItem('meetoops_lang', next);
  }

  async function handleLogout() {
    try {
      await api.logout();
    } finally {
      setCsrfToken('');
      session = null;
    }
  }

  const navGroups = $derived([
    {
      label: t.nav.overview,
      items: [
        { key: 'dashboard' as PageKey, label: t.nav.dashboard, icon: LayoutDashboard },
        { key: 'monitoring' as PageKey, label: t.nav.monitoring, icon: Activity },
        { key: 'alerts' as PageKey, label: t.nav.alerts, icon: Bell },
      ],
    },
    {
      label: t.nav.docker,
      items: [
        { key: 'containers' as PageKey, label: t.nav.containers, icon: Box },
        { key: 'images' as PageKey, label: t.nav.images, icon: Layers },
        { key: 'volumes' as PageKey, label: t.nav.volumes, icon: HardDrive },
        { key: 'networks' as PageKey, label: t.nav.networks, icon: Network },
        { key: 'stacks' as PageKey, label: t.nav.stacks, icon: FileCode2 },
      ],
    },
    {
      label: t.nav.dataServices,
      items: [
        { key: 'discovery' as PageKey, label: t.nav.discovery, icon: Radar },
        { key: 'databases' as PageKey, label: t.nav.databases, icon: Database },
        { key: 'redis' as PageKey, label: t.nav.redis, icon: KeyRound },
      ],
    },
    {
      label: t.nav.operations,
      items: [
        { key: 'logs' as PageKey, label: t.nav.logs, icon: ScrollText },
        { key: 'terminal' as PageKey, label: t.nav.terminal, icon: Terminal },
        { key: 'servers' as PageKey, label: t.nav.servers, icon: Server },
      ],
    },
    {
      label: t.nav.system,
      items: [{ key: 'settings' as PageKey, label: t.nav.settings, icon: Settings }],
    },
  ]);
</script>

{#if bootLoading}
  <div class="h-screen overflow-hidden flex bg-[var(--background)]">
    <div class="w-60 shrink-0 bg-[var(--sidebar)] border-r border-[var(--border)] p-4 space-y-5">
      <div class="flex items-center gap-2.5">
        <Skeleton class="w-7 h-7 rounded-[var(--radius-md)]" />
        <Skeleton class="h-4 w-24" />
      </div>
      <div class="space-y-2.5">
        {#each [1, 2, 3, 4, 5, 6, 7, 8] as _}
          <Skeleton class="h-7 w-full rounded-[var(--radius-md)]" />
        {/each}
      </div>
    </div>
    <div class="flex-1 flex flex-col">
      <div class="h-12 px-5 bg-[var(--sidebar)] border-b border-[var(--border)] flex items-center justify-between">
        <Skeleton class="h-4 w-24" />
        <Skeleton class="h-7 w-40 rounded-[var(--radius-md)]" />
      </div>
      <div class="p-6 space-y-5">
        <Skeleton class="h-6 w-52" />
        <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
          {#each [1, 2, 3, 4] as _}
            <Skeleton class="h-24 w-full rounded-[var(--radius-md)]" />
          {/each}
        </div>
        <Skeleton class="h-64 w-full rounded-[var(--radius-md)]" />
      </div>
    </div>
  </div>
{:else if bootError}
  <div class="min-h-screen bg-[var(--background)] flex flex-col items-center justify-center gap-3 px-4">
    <div class="text-sm font-medium text-[var(--danger)]">{bootError}</div>
    <button
      type="button"
      onclick={bootstrap}
      class="px-3.5 py-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)]"
    >
      {t.retry}
    </button>
  </div>
{:else if setupStatus && !setupStatus.initialized}
  <SetupWizard
    initialStage={setupStatus.setup_stage}
    {lang}
    {theme}
    onLangChange={changeLang}
    onThemeChange={changeTheme}
    onComplete={async (sess) => {
      if (sess) session = sess;
      await bootstrap();
    }}
  />
{:else if !session}
  <LoginView
    {lang}
    {theme}
    {appName}
    onLangChange={changeLang}
    onThemeChange={changeTheme}
    onLogin={(sess) => {
      session = sess;
    }}
  />
{:else}
  <div class="h-screen overflow-hidden flex bg-[var(--background)] text-[var(--foreground)]">
    <!-- Collapsible Sidebar with Independent Scroll -->
    <aside
      class="h-full shrink-0 bg-[var(--sidebar)] border-r border-[var(--border)] flex flex-col transition-all duration-150 {sidebarCollapsed ? 'w-14' : 'w-60'}"
    >
      <div class="h-12 shrink-0 px-3 border-b border-[var(--border)] flex items-center justify-between">
        <div class="flex items-center gap-2.5 overflow-hidden">
          <div class="w-7 h-7 shrink-0 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] flex items-center justify-center font-bold text-xs">
            M
          </div>
          {#if !sidebarCollapsed}
            <span class="font-semibold text-sm truncate">{appName}</span>
          {/if}
        </div>
        <button
          type="button"
          onclick={() => (sidebarCollapsed = !sidebarCollapsed)}
          class="p-1 rounded text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)]"
          title="Toggle Sidebar"
        >
          {#if sidebarCollapsed}
            <PanelLeftOpen class="w-4 h-4" />
          {:else}
            <PanelLeftClose class="w-4 h-4" />
          {/if}
        </button>
      </div>

      <nav class="flex-1 overflow-y-auto py-3 px-2 space-y-4">
        {#each navGroups as group}
          <div>
            {#if !sidebarCollapsed}
              <div class="px-2 pb-1 text-[10px] font-medium uppercase tracking-wider text-[var(--subtle-foreground)]">
                {group.label}
              </div>
            {/if}
            <div class="space-y-0.5">
              {#each group.items as item}
                {@const Icon = item.icon}
                <button
                  type="button"
                  onclick={() => navigate(item.key)}
                  title={item.label}
                  class="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-[var(--radius-md)] text-xs transition-colors {activePage === item.key ? 'bg-[var(--primary-subtle)] text-[var(--primary)] font-medium' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface-elevated)]'}"
                >
                  <Icon class="w-4 h-4 shrink-0" />
                  {#if !sidebarCollapsed}
                    <span class="truncate">{item.label}</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </nav>

      <div class="p-2 border-t border-[var(--border)]">
        <button
          type="button"
          onclick={handleLogout}
          class="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-[var(--radius-md)] text-xs text-[var(--muted-foreground)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-colors"
        >
          <LogOut class="w-4 h-4 shrink-0" />
          {#if !sidebarCollapsed}
            <span>{t.nav.signOut}</span>
          {/if}
        </button>
      </div>
    </aside>

    <!-- Main Workspace Column with Fixed Navbar + Independent Scroll -->
    <div class="flex-1 h-full flex flex-col min-w-0 overflow-hidden">
      <header class="h-12 shrink-0 px-5 bg-[var(--sidebar)] border-b border-[var(--border)] flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
            <span class="w-2 h-2 rounded-full bg-[var(--success)]"></span>
            <span class="font-mono tabular-nums">:{setupStatus?.active_port ?? 9480}</span>
          </div>

          <div class="w-44">
            <Select
              options={serverOptions}
              value={activeServerId}
              onchange={(val) => (activeServerId = val)}
              searchable={serverOptions.length > 4}
              placeholder="Select Node"
            />
          </div>

          <button
            type="button"
            onclick={() => (commandPaletteOpen = true)}
            class="hidden sm:flex h-[34px] items-center gap-2 px-2.5 rounded-[var(--radius-md)] border border-[var(--border-strong)] bg-[var(--surface)] text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:border-[var(--primary)]/50 transition-colors"
            title="Quick Jump (Ctrl+K)"
          >
            <Search class="w-3.5 h-3.5" />
            <span>Quick jump…</span>
            <kbd class="ml-1 px-1 py-0.5 text-[10px] font-mono rounded bg-[var(--surface-secondary)] border border-[var(--border)]">
              Ctrl+K
            </kbd>
          </button>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            onclick={() => navigate('alerts')}
            class="relative h-[34px] w-[34px] inline-flex items-center justify-center rounded-[var(--radius-md)] border border-[var(--border-strong)] bg-[var(--surface)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
            title={firingAlertsCount > 0 ? `${firingAlertsCount} firing alert(s)` : 'Alerts'}
          >
            <Bell class="w-4 h-4" />
            {#if firingAlertsCount > 0}
              <span class="absolute -top-1 -right-1 min-w-4 h-4 px-1 rounded-full bg-[var(--danger)] text-white text-[10px] font-bold flex items-center justify-center tabular-nums">
                {firingAlertsCount > 99 ? '99+' : firingAlertsCount}
              </span>
            {/if}
          </button>

          <ThemeToggle {theme} onChange={changeTheme} />
          <div class="flex items-center gap-1 bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-md)] p-0.5">
            <Globe class="w-3.5 h-3.5 text-[var(--muted-foreground)] ml-1" />
            <button
              type="button"
              class="px-1.5 py-0.5 text-[11px] rounded-[var(--radius-sm)] {lang === 'en' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)]'}"
              onclick={() => changeLang('en')}
            >
              EN
            </button>
            <button
              type="button"
              class="px-1.5 py-0.5 text-[11px] rounded-[var(--radius-sm)] {lang === 'id' ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)]'}"
              onclick={() => changeLang('id')}
            >
              ID
            </button>
          </div>

          <div class="text-xs text-right">
            <div class="font-medium text-[var(--foreground)] leading-tight">{session.user.full_name}</div>
            <div class="text-[10px] font-mono text-[var(--muted-foreground)]">{session.user.role}</div>
          </div>
        </div>
      </header>

      <main class="flex-1 overflow-y-auto p-6">
        {#if activePage === 'dashboard'}
          <DashboardView {lang} />
        {:else if activePage === 'settings'}
          <SettingsView
            {lang}
            canWriteSettings={session.permissions.includes('settings:write')}
            onLangChange={changeLang}
            onAppNameChange={(name) => (appName = name)}
          />
        {:else if activePage === 'containers'}
          <ContainersView
            {lang}
            canOperate={session.permissions.includes('docker:operate')}
            canDelete={session.permissions.includes('docker:delete')}
            canRevealSecrets={session.permissions.includes('docker:reveal_secrets')}
            onOpenLogs={(cid) => {
              selectedLogContainerId = cid;
              navigate('logs');
            }}
            onOpenTerminal={(cid) => {
              selectedTerminalContainerId = cid;
              navigate('terminal');
            }}
          />
        {:else if activePage === 'images'}
          <ImagesView
            {lang}
            canOperate={session.permissions.includes('docker:operate')}
            canDelete={session.permissions.includes('docker:delete')}
          />
        {:else if activePage === 'volumes'}
          <VolumesView
            {lang}
            canOperate={session.permissions.includes('docker:operate')}
            canDelete={session.permissions.includes('docker:delete')}
          />
        {:else if activePage === 'networks'}
          <NetworksView
            {lang}
            canOperate={session.permissions.includes('docker:operate')}
            canDelete={session.permissions.includes('docker:delete')}
          />
        {:else if activePage === 'logs'}
          <LogCenterView {lang} initialContainerId={selectedLogContainerId} />
        {:else if activePage === 'monitoring'}
          <MonitoringView {lang} {theme} serverId={activeServerId} />
        {:else if activePage === 'discovery'}
          <DiscoveryView
            {lang}
            canManageCredentials={session.permissions.includes('services:credentials')}
            onOpenDbExplorer={(sid) => {
              selectedServiceId = sid;
              navigate('databases');
            }}
            onOpenRedisExplorer={(sid) => {
              selectedServiceId = sid;
              navigate('redis');
            }}
          />
        {:else if activePage === 'databases'}
          <DatabaseExplorerView {lang} initialServiceId={selectedServiceId} />
        {:else if activePage === 'redis'}
          <RedisExplorerView
            {lang}
            initialServiceId={selectedServiceId}
            canWrite={session.permissions.includes('redis:write')}
          />
        {:else if activePage === 'terminal'}
          <TerminalView
            {lang}
            canExec={session.permissions.includes('terminal:exec')}
            initialContainerId={selectedTerminalContainerId}
          />
        {:else if activePage === 'stacks'}
          <StacksView {lang} canOperate={session.permissions.includes('docker:operate')} />
        {:else if activePage === 'servers'}
          <ServersView
            {lang}
            canWrite={session.permissions.includes('servers:write')}
            canDelete={session.permissions.includes('servers:delete')}
          />
        {:else if activePage === 'alerts'}
          <AlertsView {lang} canWrite={session.permissions.includes('alerts:write')} />
        {/if}
      </main>
    </div>
  </div>

  <CommandPalette
    open={commandPaletteOpen}
    onClose={() => (commandPaletteOpen = false)}
    onNavigate={(page) => {
      navigate(page as PageKey);
    }}
  />
{/if}

<svelte:window onkeydown={handleGlobalKeydown} />
<ToastContainer />
<ConfirmModal />
