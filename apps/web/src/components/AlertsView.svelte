<script lang="ts">
  import {
    api,
    type AlertChannelDto,
    type AlertEventDto,
    type AlertRuleDto,
    type PaginationMeta,
  } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import { confirmDialog } from '../lib/confirm.svelte';
  import { toast } from '../lib/toast.svelte';
  import Select from './ui/Select.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import DateRangePicker from './ui/DateRangePicker.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import ExplorerTabs from './ui/ExplorerTabs.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import { Bell, Plus, Trash2, CheckCircle2, AlertCircle, Search, Send, Webhook, History, SlidersHorizontal } from 'lucide-svelte';

  interface Props {
    lang: Language;
    canWrite: boolean;
  }

  let { lang, canWrite }: Props = $props();

  let tab = $state<'events' | 'rules' | 'channels'>('events');
  let errorMsg = $state<string | null>(null);

  // Events (Server-side paginated)
  let events = $state<AlertEventDto[]>([]);
  let meta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let loadingEvents = $state(true);
  let search = $state('');
  let debouncedSearch = $state('');
  let severities = $state<string[]>([]);
  let statuses = $state<string[]>([]);
  let from = $state('');
  let to = $state('');
  let sortBy = $state('triggered_at');
  let sortDir = $state<'asc' | 'desc'>('desc');
  let page = $state(1);
  let perPage = $state(10);

  // Rules
  let rules = $state<AlertRuleDto[]>([]);
  let loadingRules = $state(false);
  let ruleName = $state('');
  let ruleMetric = $state('cpu_percent');
  let ruleOp = $state('gte');
  let ruleThreshold = $state(85);
  let ruleSeverity = $state('critical');

  // Notification Channels
  let channels = $state<AlertChannelDto[]>([]);
  let loadingChannels = $state(false);
  let chName = $state('');
  let chType = $state<'webhook' | 'slack' | 'discord' | 'telegram'>('webhook');
  let chUrl = $state('');
  let testingChannelId = $state<string | null>(null);

  $effect(() => {
    const q = search;
    const t = setTimeout(() => (debouncedSearch = q), 300);
    return () => clearTimeout(t);
  });

  async function loadEvents() {
    loadingEvents = true;
    errorMsg = null;
    try {
      const res = await api.queryAlertEvents({
        search: debouncedSearch || undefined,
        severity: severities.join(',') || undefined,
        status: statuses.join(',') || undefined,
        from: from || undefined,
        to: to || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        per_page: perPage,
      });
      events = res.items;
      meta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load alert events.';
    } finally {
      loadingEvents = false;
    }
  }

  async function loadRules() {
    loadingRules = true;
    errorMsg = null;
    try {
      rules = await api.listAlertRules();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load alert rules.';
    } finally {
      loadingRules = false;
    }
  }

  async function loadChannels() {
    loadingChannels = true;
    errorMsg = null;
    try {
      channels = await api.listAlertChannels();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load notification channels.';
    } finally {
      loadingChannels = false;
    }
  }

  function handleSort(col: string) {
    if (sortBy === col) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    else {
      sortBy = col;
      sortDir = 'desc';
    }
    page = 1;
  }

  async function handleCreateRule(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    try {
      const name = ruleName;
      await api.createAlertRule({
        name: ruleName,
        metric_type: ruleMetric,
        operator: ruleOp,
        threshold: Number(ruleThreshold),
        severity: ruleSeverity,
      });
      toast.success('Alert rule created', name);
      ruleName = '';
      await loadRules();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create alert rule.';
      errorMsg = msg;
      toast.error('Rule creation failed', msg);
    }
  }

  async function handleToggleRule(r: AlertRuleDto, next: boolean) {
    try {
      await api.toggleAlertRule(r.id, next);
      toast.info(next ? 'Rule enabled' : 'Rule disabled', r.name);
      await loadRules();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to update rule.';
    }
  }

  async function handleDeleteRule(r: AlertRuleDto) {
    const ok = await confirmDialog({
      title: 'Delete Alert Rule',
      message: 'Remove this threshold alert rule?',
      targetName: r.name,
      confirmText: 'Delete Rule',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteAlertRule(r.id);
      toast.success('Alert rule deleted', r.name);
      await loadRules();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to delete rule.';
    }
  }

  async function handleCreateChannel(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = null;
    try {
      const name = chName;
      await api.createAlertChannel({ name: chName, channel_type: chType, target_url: chUrl });
      toast.success('Notification channel added', name);
      chName = '';
      chUrl = '';
      await loadChannels();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create channel.';
      errorMsg = msg;
      toast.error('Channel creation failed', msg);
    }
  }

  async function handleToggleChannel(c: AlertChannelDto, next: boolean) {
    try {
      await api.toggleAlertChannel(c.id, next);
      toast.info(next ? 'Channel enabled' : 'Channel paused', c.name);
      await loadChannels();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to update channel.';
    }
  }

  async function handleTestChannel(c: AlertChannelDto) {
    testingChannelId = c.id;
    errorMsg = null;
    try {
      await api.testAlertChannel(c.id);
      toast.success('Test alert delivered', c.name);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Webhook test failed.';
      errorMsg = msg;
      toast.error('Test delivery failed', msg);
    } finally {
      testingChannelId = null;
    }
  }

  async function handleDeleteChannel(c: AlertChannelDto) {
    const ok = await confirmDialog({
      title: 'Delete Notification Channel',
      message: 'Remove this outbound notification channel?',
      targetName: `${c.name} (${c.channel_type})`,
      confirmText: 'Delete Channel',
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await api.deleteAlertChannel(c.id);
      toast.success('Channel deleted', c.name);
      await loadChannels();
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to delete channel.';
    }
  }

  async function ackEvent(id: string) {
    await api.ackAlertEvent(id);
    toast.info('Alert acknowledged');
    await loadEvents();
  }

  async function resolveEvent(id: string) {
    await api.resolveAlertEvent(id);
    toast.success('Alert resolved');
    await loadEvents();
  }

  $effect(() => {
    void debouncedSearch;
    void severities;
    void statuses;
    void from;
    void to;
    void sortBy;
    void sortDir;
    void page;
    void perPage;
    if (tab === 'events') loadEvents();
  });

  $effect(() => {
    if (tab === 'rules') loadRules();
    if (tab === 'channels') loadChannels();
  });
</script>

<div class="w-full space-y-4">
  <div>
    <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
      <Bell class="w-5 h-5 text-[var(--primary)]" />
      {lang === 'id' ? 'Peringatan & Aturan Alert' : 'Alerts & Threshold Rules'}
    </h1>
    <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
      Evaluated automatically against live host telemetry with deduplication and auto-resolve.
    </p>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <ExplorerTabs
    bind:active={tab}
    bodyClass="p-5 space-y-4"
    tabs={[
      { id: 'events', label: 'Alert History', icon: History, badge: meta.total > 0 ? meta.total : undefined },
      { id: 'rules', label: 'Threshold Rules', icon: SlidersHorizontal, badge: rules.length > 0 ? rules.length : undefined },
      { id: 'channels', label: 'Notification Channels', icon: Webhook, badge: channels.length > 0 ? channels.length : undefined },
    ]}
  >
  {#if tab === 'events'}
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-[var(--muted-foreground)] absolute left-2.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder="Search rule or message…"
          bind:value={search}
          oninput={() => (page = 1)}
          class="pl-8 pr-3 py-1.5 w-60 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs"
        />
      </div>
      <MultiSelect
        class="w-40"
        searchable={false}
        placeholder="Severity"
        bind:values={severities}
        onchange={() => (page = 1)}
        options={[
          { value: 'critical', label: 'Critical' },
          { value: 'warning', label: 'Warning' },
          { value: 'info', label: 'Info' },
        ]}
      />
      <MultiSelect
        class="w-40"
        searchable={false}
        placeholder="Status"
        bind:values={statuses}
        onchange={() => (page = 1)}
        options={[
          { value: 'firing', label: 'Firing' },
          { value: 'acknowledged', label: 'Acknowledged' },
          { value: 'resolved', label: 'Resolved' },
        ]}
      />
      <DateRangePicker bind:from bind:to onchange={() => (page = 1)} />
    </div>

    <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
      {#if loadingEvents && events.length === 0}
        <TableSkeleton rows={5} columns={6} />
      {:else if events.length === 0}
        <div class="p-10 text-center text-xs text-[var(--muted-foreground)]">
          No alert events recorded. Create a threshold rule in the "Threshold Rules" tab to start monitoring.
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
                <th class="px-4 py-2.5">
                  <SortHeader label="Triggered" column="triggered_at" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
                </th>
                <th class="px-4 py-2.5">
                  <SortHeader label="Rule" column="rule_name" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
                </th>
                <th class="px-4 py-2.5">
                  <SortHeader label="Severity" column="severity" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
                </th>
                <th class="px-4 py-2.5">
                  <SortHeader label="Status" column="status" activeColumn={sortBy} direction={sortDir} onSort={handleSort} />
                </th>
                <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Message</th>
                <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)] text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-[var(--border-subtle)]">
              {#each events as ev}
                <tr class="hover:bg-[var(--surface-elevated)]">
                  <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] tabular-nums whitespace-nowrap">
                    {new Date(ev.triggered_at).toLocaleString()}
                  </td>
                  <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">{ev.rule_name}</td>
                  <td class="px-4 py-2.5">
                    <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase {ev.severity === 'critical' ? 'bg-[var(--danger)] text-white' : ev.severity === 'warning' ? 'bg-[var(--warning)] text-white' : 'bg-[var(--info)] text-white'}">
                      {ev.severity}
                    </span>
                  </td>
                  <td class="px-4 py-2.5 font-mono text-[11px]">{ev.status}</td>
                  <td class="px-4 py-2.5 text-[var(--muted-foreground)]">{ev.message}</td>
                  <td class="px-4 py-2.5 text-right whitespace-nowrap">
                    {#if canWrite && ev.status !== 'resolved'}
                      <div class="inline-flex gap-1.5">
                        {#if ev.status === 'firing'}
                          <button type="button" onclick={() => ackEvent(ev.id)} class="px-2 py-1 rounded bg-[var(--warning-subtle)] text-[var(--warning)] text-[11px] font-medium">
                            Ack
                          </button>
                        {/if}
                        <button type="button" onclick={() => resolveEvent(ev.id)} class="px-2 py-1 rounded bg-[var(--success-subtle)] text-[var(--success)] text-[11px] font-medium inline-flex items-center gap-1">
                          <CheckCircle2 class="w-3 h-3" /> Resolve
                        </button>
                      </div>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      <PaginationBar
        page={meta.page}
        perPage={meta.per_page}
        total={meta.total}
        totalPages={meta.total_pages}
        onPageChange={(p) => (page = p)}
        onPerPageChange={(pp) => {
          perPage = pp;
          page = 1;
        }}
      />
    </div>
  {:else if tab === 'rules'}
    {#if canWrite}
      <form onsubmit={handleCreateRule} class="w-full p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] grid grid-cols-1 md:grid-cols-6 gap-3 items-end">
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Rule Name</span>
          <input type="text" required placeholder="High RAM Alert" bind:value={ruleName} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Metric</span>
          <Select
            class="w-full"
            searchable={false}
            bind:value={ruleMetric}
            options={[
              { value: 'cpu_percent', label: 'CPU %' },
              { value: 'mem_percent', label: 'Memory %' },
              { value: 'disk_percent', label: 'Disk Space %' },
              { value: 'disk_active_percent', label: 'Disk Active I/O %' },
            ]}
          />
        </div>
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Condition</span>
          <Select
            class="w-full"
            searchable={false}
            bind:value={ruleOp}
            options={[
              { value: 'gte', label: '>=' },
              { value: 'gt', label: '>' },
              { value: 'lt', label: '<' },
            ]}
          />
        </div>
        <label class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Threshold (%)</span>
          <input type="number" step="0.5" required bind:value={ruleThreshold} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs tabular-nums" />
        </label>
        <div class="space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Severity</span>
          <Select
            class="w-full"
            searchable={false}
            bind:value={ruleSeverity}
            options={[
              { value: 'warning', label: 'Warning' },
              { value: 'critical', label: 'Critical' },
              { value: 'info', label: 'Info' },
            ]}
          />
        </div>
        <button type="submit" class="h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center justify-center gap-1.5">
          <Plus class="w-3.5 h-3.5" /> Add Rule
        </button>
      </form>
    {/if}

    <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
      {#if loadingRules && rules.length === 0}
        <TableSkeleton rows={4} columns={5} />
      {:else if rules.length === 0}
        <div class="p-8 text-center text-xs text-[var(--muted-foreground)]">No alert rules configured yet.</div>
      {:else}
        <table class="w-full text-left border-collapse text-xs">
          <thead>
            <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
              <th class="px-4 py-2.5 font-medium">Enabled</th>
              <th class="px-4 py-2.5 font-medium">Name</th>
              <th class="px-4 py-2.5 font-medium">Condition</th>
              <th class="px-4 py-2.5 font-medium">Severity</th>
              <th class="px-4 py-2.5 font-medium text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--border-subtle)]">
            {#each rules as r}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2.5">
                  <Checkbox checked={r.is_enabled} disabled={!canWrite} onchange={(v) => handleToggleRule(r, v)} />
                </td>
                <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">{r.name}</td>
                <td class="px-4 py-2.5 font-mono text-[var(--muted-foreground)]">
                  {r.metric_type} {r.operator} {r.threshold}%
                </td>
                <td class="px-4 py-2.5 uppercase font-bold text-[10px]">
                  <span class="px-2 py-0.5 rounded {r.severity === 'critical' ? 'bg-[var(--danger)] text-white' : r.severity === 'warning' ? 'bg-[var(--warning)] text-white' : 'bg-[var(--info)] text-white'}">
                    {r.severity}
                  </span>
                </td>
                <td class="px-4 py-2.5 text-right">
                  {#if canWrite}
                    <button type="button" onclick={() => handleDeleteRule(r)} class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]" title="Delete Rule">
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {:else}
    {#if canWrite}
      <form onsubmit={handleCreateChannel} class="w-full p-4 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] grid grid-cols-1 md:grid-cols-12 gap-3 items-end">
        <label class="md:col-span-3 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Channel Name</span>
          <input type="text" required placeholder="Ops Slack #alerts" bind:value={chName} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] text-xs" />
        </label>
        <div class="md:col-span-2 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)] block">Provider</span>
          <Select
            class="w-full"
            searchable={false}
            bind:value={chType}
            options={[
              { value: 'webhook', label: 'Generic Webhook' },
              { value: 'slack', label: 'Slack Webhook' },
              { value: 'discord', label: 'Discord Webhook' },
              { value: 'telegram', label: 'Telegram Bot URL' },
            ]}
          />
        </div>
        <label class="md:col-span-5 space-y-1">
          <span class="text-[11px] text-[var(--muted-foreground)]">Target Webhook URL</span>
          <input type="url" required placeholder="https://hooks.slack.com/services/..." bind:value={chUrl} class="w-full px-3 py-1.5 rounded bg-[var(--background)] border border-[var(--border-strong)] font-mono text-xs" />
        </label>
        <button type="submit" class="md:col-span-2 h-[34px] px-3 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-xs font-medium inline-flex items-center justify-center gap-1.5">
          <Plus class="w-3.5 h-3.5" /> Add Channel
        </button>
      </form>
    {/if}

    <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
      {#if loadingChannels && channels.length === 0}
        <TableSkeleton rows={3} columns={5} />
      {:else if channels.length === 0}
        <div class="p-8 text-center text-xs text-[var(--muted-foreground)]">
          No outbound notification channels configured. Add a Slack, Discord, Telegram, or generic Webhook endpoint to receive real-time alert notifications.
        </div>
      {:else}
        <table class="w-full text-left border-collapse text-xs">
          <thead>
            <tr class="bg-[var(--surface-secondary)] border-b border-[var(--border)] text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
              <th class="px-4 py-2.5 font-medium">Enabled</th>
              <th class="px-4 py-2.5 font-medium">Name</th>
              <th class="px-4 py-2.5 font-medium">Provider</th>
              <th class="px-4 py-2.5 font-medium">Target URL</th>
              <th class="px-4 py-2.5 font-medium text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--border-subtle)]">
            {#each channels as c}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2.5">
                  <Checkbox checked={c.is_enabled} disabled={!canWrite} onchange={(v) => handleToggleChannel(c, v)} />
                </td>
                <td class="px-4 py-2.5 font-medium text-[var(--foreground)]">{c.name}</td>
                <td class="px-4 py-2.5">
                  <span class="px-2 py-0.5 rounded bg-[var(--primary-subtle)] text-[var(--primary)] font-mono text-[10px] uppercase font-bold">
                    {c.channel_type}
                  </span>
                </td>
                <td class="px-4 py-2.5 font-mono text-[11px] text-[var(--muted-foreground)] max-w-[320px] truncate">
                  {c.target_url}
                </td>
                <td class="px-4 py-2.5 text-right whitespace-nowrap">
                  {#if canWrite}
                    <div class="inline-flex items-center gap-1.5">
                      <button
                        type="button"
                        disabled={testingChannelId === c.id}
                        onclick={() => handleTestChannel(c)}
                        class="px-2.5 py-1 rounded bg-[var(--surface-secondary)] hover:bg-[var(--surface-elevated)] border border-[var(--border)] text-[11px] font-medium inline-flex items-center gap-1 disabled:opacity-50"
                      >
                        <Send class="w-3 h-3" /> {testingChannelId === c.id ? 'Sending…' : 'Send Test'}
                      </button>
                      <button
                        type="button"
                        onclick={() => handleDeleteChannel(c)}
                        class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"
                        title="Delete Channel"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
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
  {/if}
  </ExplorerTabs>
</div>
