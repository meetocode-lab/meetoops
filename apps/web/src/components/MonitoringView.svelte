<script lang="ts">
  import type * as echarts from 'echarts';
  import { api, type HostMetricPoint, type PaginationMeta } from '../lib/api';
  import type { Language } from '../lib/i18n';
  import type { ThemePreference } from '../lib/theme';
  import { resolveTheme } from '../lib/theme';
  import EChart from './EChart.svelte';
  import MultiSelect from './ui/MultiSelect.svelte';
  import DateRangePicker from './ui/DateRangePicker.svelte';
  import PaginationBar from './ui/PaginationBar.svelte';
  import SortHeader from './ui/SortHeader.svelte';
  import Skeleton from './ui/Skeleton.svelte';
  import TableSkeleton from './ui/TableSkeleton.svelte';
  import {
    Activity,
    Cpu,
    MemoryStick,
    HardDrive,
    ArrowDownUp,
    ExternalLink,
    AlertCircle,
    History,
    RefreshCw,
  } from 'lucide-svelte';

  interface Props {
    lang: Language;
    theme: ThemePreference;
    serverId?: string;
  }

  let { lang, theme, serverId = 'local' }: Props = $props();

  type RangeKey = '15m' | '1h' | '6h' | '24h' | '7d';
  type RiskLevel = 'safe' | 'warning' | 'critical' | 'unknown';
  const ranges: RangeKey[] = ['15m', '1h', '6h', '24h', '7d'];

  let range = $state<RangeKey>('1h');
  let points = $state<HostMetricPoint[]>([]);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let wsLive = $state(false);

  // Server-side paginated history log state (default 10 rows, newest on top)
  let logItems = $state<HostMetricPoint[]>([]);
  let logMeta = $state<PaginationMeta>({ page: 1, per_page: 10, total: 0, total_pages: 1 });
  let logLoading = $state(false);
  let logRisks = $state<string[]>([]);
  let logFrom = $state('');
  let logTo = $state('');
  let logSortBy = $state('recorded_at');
  let logSortDir = $state<'asc' | 'desc'>('desc');
  let logPage = $state(1);
  let logPerPage = $state(10);

  async function loadServerLogs() {
    logLoading = true;
    try {
      const res = await api.queryHostMetricLogs({
        server_id: serverId,
        risk: logRisks.join(','),
        from: logFrom || undefined,
        to: logTo || undefined,
        sort_by: logSortBy,
        sort_dir: logSortDir,
        page: logPage,
        per_page: logPerPage,
      });
      logItems = res.items;
      logMeta = res.meta;
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load server metric logs.';
    } finally {
      logLoading = false;
    }
  }

  function handleLogSort(col: string) {
    if (logSortBy === col) {
      logSortDir = logSortDir === 'asc' ? 'desc' : 'asc';
    } else {
      logSortBy = col;
      logSortDir = 'desc';
    }
    logPage = 1;
  }

  $effect(() => {
    void serverId;
    void logRisks;
    void logFrom;
    void logTo;
    void logSortBy;
    void logSortDir;
    void logPage;
    void logPerPage;
    loadServerLogs();
  });

  function rowRisk(p: HostMetricPoint): RiskLevel {
    const c = evalRisk(p.cpu_percent, 70, 85);
    const m = evalRisk(p.mem_percent, 75, 90);
    const d = evalRisk(p.disk_percent, 75, 90);
    if (c === 'critical' || m === 'critical' || d === 'critical') return 'critical';
    if (c === 'warning' || m === 'warning' || d === 'warning') return 'warning';
    return 'safe';
  }

  const isDark = $derived(resolveTheme(theme) === 'dark');

  const palette = $derived(
    isDark
      ? {
          text: '#cbd5e1',
          grid: '#1e3552',
          tooltipBg: '#0d1d33',
          tooltipBorder: '#2c4b70',
          primary: '#3b82f6',
          cyan: '#22d3ee',
          success: '#4ade80',
          warning: '#fbbf24',
        }
      : {
          text: '#475569',
          grid: '#dbeafe',
          tooltipBg: '#ffffff',
          tooltipBorder: '#bfdbfe',
          primary: '#2563eb',
          cyan: '#0891b2',
          success: '#15803d',
          warning: '#b45309',
        },
  );

  const latest = $derived(points.length > 0 ? points[points.length - 1] : null);

  function fmtPct(v: number | null | undefined, digits = 1): string {
    if (v === null || v === undefined || Number.isNaN(v)) return '—';
    return `${v.toFixed(digits)}%`;
  }

  function round1(v: number | null): number | null {
    if (v === null || Number.isNaN(v)) return null;
    return Number(v.toFixed(1));
  }

  function evalRisk(val: number | null | undefined, warnAt: number, critAt: number): RiskLevel {
    if (val === null || val === undefined || Number.isNaN(val)) return 'unknown';
    if (val >= critAt) return 'critical';
    if (val >= warnAt) return 'warning';
    return 'safe';
  }

  const cpuRisk = $derived(evalRisk(latest?.cpu_percent, 70, 85));
  const memRisk = $derived(evalRisk(latest?.mem_percent, 75, 90));
  const diskRisk = $derived(
    latest?.disk_active_percent !== null && latest?.disk_active_percent !== undefined
      ? evalRisk(latest.disk_active_percent, 80, 95)
      : evalRisk(latest?.disk_percent, 75, 90),
  );
  const netRisk = $derived<RiskLevel>(
    !wsLive
      ? 'critical'
      : latest?.net_rx_bps === null && latest?.net_tx_bps === null
        ? 'warning'
        : 'safe',
  );

  function riskCardClasses(level: RiskLevel): string {
    if (level === 'critical')
      return 'bg-[var(--danger-subtle)] border-[var(--danger)]';
    if (level === 'warning')
      return 'bg-[var(--warning-subtle)] border-[var(--warning)]';
    if (level === 'safe')
      return 'bg-[var(--success-subtle)] border-[var(--success)]/60';
    return 'bg-[var(--surface)] border-[var(--border)]';
  }

  function riskTextClasses(level: RiskLevel): string {
    if (level === 'critical') return 'text-[var(--danger)] font-bold';
    if (level === 'warning') return 'text-[var(--warning)] font-semibold';
    if (level === 'safe') return 'text-[var(--success)] font-medium';
    return 'text-[var(--muted-foreground)]';
  }

  function rowTriggers(p: HostMetricPoint): Array<{ label: string; level: RiskLevel }> {
    const out: Array<{ label: string; level: RiskLevel }> = [];
    const c = evalRisk(p.cpu_percent, 70, 85);
    const m = evalRisk(p.mem_percent, 75, 90);
    const d = evalRisk(p.disk_percent, 75, 90);
    const dio = evalRisk(p.disk_active_percent, 80, 95);
    if (c === 'warning' || c === 'critical') out.push({ label: `CPU ${fmtPct(p.cpu_percent)}`, level: c });
    if (m === 'warning' || m === 'critical') out.push({ label: `RAM ${fmtPct(p.mem_percent)}`, level: m });
    if (d === 'warning' || d === 'critical') out.push({ label: `Disk ${fmtPct(p.disk_percent)}`, level: d });
    if (dio === 'warning' || dio === 'critical') out.push({ label: `I/O ${fmtPct(p.disk_active_percent)}`, level: dio });
    return out;
  }

  function riskBadgeClasses(level: RiskLevel): string {
    if (level === 'critical') return 'bg-[var(--danger)] text-white';
    if (level === 'warning') return 'bg-[var(--warning)] text-white';
    if (level === 'safe') return 'bg-[var(--success)] text-white';
    return 'bg-[var(--surface-secondary)] text-[var(--muted-foreground)]';
  }

  function riskLabel(level: RiskLevel): string {
    if (lang === 'id') {
      if (level === 'critical') return 'KRITIS';
      if (level === 'warning') return 'WASPADA';
      if (level === 'safe') return 'AMAN';
      return 'N/A';
    }
    if (level === 'critical') return 'CRITICAL';
    if (level === 'warning') return 'WARNING';
    if (level === 'safe') return 'SAFE';
    return 'N/A';
  }

  async function loadHistory(r: RangeKey, sid: string) {
    loading = true;
    errorMsg = null;
    try {
      points = await api.hostMetricsHistory(r, sid);
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Failed to load monitoring history.';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadHistory(range, serverId);
  });

  $effect(() => {
    const targetServer = serverId;
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${proto}//${window.location.host}/ws/v1/monitoring/realtime`);

    ws.onopen = () => {
      wsLive = true;
    };
    ws.onmessage = (ev) => {
      try {
        const pt = JSON.parse(ev.data) as HostMetricPoint;
        if (pt.server_id !== targetServer) return;
        const maxPoints = 500;
        if (points.length >= maxPoints) {
          points = [...points.slice(points.length - maxPoints + 1), pt];
        } else {
          points = [...points, pt];
        }

        // Real-time update for page 1 when sorted newest-first (recorded_at DESC)
        if (logPage === 1 && logSortBy === 'recorded_at' && logSortDir === 'desc' && !logTo) {
          const r = rowRisk(pt);
          const matchesRisk = logRisks.length === 0 || logRisks.includes(r);
          if (matchesRisk) {
            logItems = [pt, ...logItems].slice(0, logPerPage);
            const nextTotal = logMeta.total + 1;
            logMeta = {
              ...logMeta,
              total: nextTotal,
              total_pages: Math.max(1, Math.ceil(nextTotal / logPerPage)),
            };
          }
        }
      } catch {
        // Ignore malformed frame
      }
    };
    ws.onclose = () => {
      wsLive = false;
    };

    return () => {
      ws.close();
      wsLive = false;
    };
  });

  const timestamps = $derived(
    points.map((p) => {
      const d = new Date(p.recorded_at);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }),
  );

  const cpuMemOption = $derived<echarts.EChartsOption>({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: palette.tooltipBg,
      borderColor: palette.tooltipBorder,
      textStyle: { color: palette.text, fontSize: 12 },
    },
    legend: {
      top: 0,
      right: 4,
      textStyle: { color: palette.text, fontSize: 11 },
    },
    grid: { left: 8, right: 12, top: 32, bottom: 8, containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: timestamps,
      axisLabel: { color: palette.text, fontSize: 10 },
      axisLine: { lineStyle: { color: palette.grid } },
    },
    yAxis: {
      type: 'value',
      min: 0,
      max: 100,
      axisLabel: { color: palette.text, fontSize: 10, formatter: '{value}%' },
      splitLine: { lineStyle: { color: palette.grid, type: 'dashed' } },
    },
    series: [
      {
        name: 'CPU %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) => round1(p.cpu_percent)),
        lineStyle: { width: 2, color: palette.primary },
        itemStyle: { color: palette.primary },
        areaStyle: { opacity: 0.14, color: palette.primary },
      },
      {
        name: 'Memory %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) => round1(p.mem_percent)),
        lineStyle: { width: 2, color: palette.cyan },
        itemStyle: { color: palette.cyan },
        areaStyle: { opacity: 0.12, color: palette.cyan },
      },
    ],
  });

  const netOption = $derived<echarts.EChartsOption>({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: palette.tooltipBg,
      borderColor: palette.tooltipBorder,
      textStyle: { color: palette.text, fontSize: 12 },
    },
    legend: {
      top: 0,
      right: 4,
      textStyle: { color: palette.text, fontSize: 11 },
    },
    grid: { left: 8, right: 12, top: 32, bottom: 8, containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: timestamps,
      axisLabel: { color: palette.text, fontSize: 10 },
      axisLine: { lineStyle: { color: palette.grid } },
    },
    yAxis: {
      type: 'value',
      min: 0,
      axisLabel: { color: palette.text, fontSize: 10, formatter: '{value} KB/s' },
      splitLine: { lineStyle: { color: palette.grid, type: 'dashed' } },
    },
    series: [
      {
        name: 'RX (KB/s)',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) =>
          p.net_rx_bps !== null ? Number((p.net_rx_bps / 1024).toFixed(1)) : null,
        ),
        lineStyle: { width: 2, color: palette.success },
        itemStyle: { color: palette.success },
        areaStyle: { opacity: 0.1, color: palette.success },
      },
      {
        name: 'TX (KB/s)',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) =>
          p.net_tx_bps !== null ? Number((p.net_tx_bps / 1024).toFixed(1)) : null,
        ),
        lineStyle: { width: 2, color: palette.warning },
        itemStyle: { color: palette.warning },
        areaStyle: { opacity: 0.1, color: palette.warning },
      },
    ],
  });

  const diskOption = $derived<echarts.EChartsOption>({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: palette.tooltipBg,
      borderColor: palette.tooltipBorder,
      textStyle: { color: palette.text, fontSize: 12 },
    },
    legend: {
      top: 0,
      right: 4,
      textStyle: { color: palette.text, fontSize: 11 },
    },
    grid: { left: 8, right: 12, top: 32, bottom: 8, containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: timestamps,
      axisLabel: { color: palette.text, fontSize: 10 },
      axisLine: { lineStyle: { color: palette.grid } },
    },
    yAxis: {
      type: 'value',
      min: 0,
      max: 100,
      axisLabel: { color: palette.text, fontSize: 10, formatter: '{value}%' },
      splitLine: { lineStyle: { color: palette.grid, type: 'dashed' } },
    },
    series: [
      {
        name: 'Active Time %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) => round1(p.disk_active_percent)),
        lineStyle: { width: 2, color: palette.success },
        itemStyle: { color: palette.success },
        areaStyle: { opacity: 0.14, color: palette.success },
      },
      {
        name: 'Space Used %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        connectNulls: false,
        data: points.map((p) => round1(p.disk_percent)),
        lineStyle: { width: 2, type: 'dashed', color: palette.primary },
        itemStyle: { color: palette.primary },
      },
    ],
  });

  function formatRate(bps: number | null): string {
    if (bps === null) return '—';
    const kb = bps / 1024;
    return kb >= 1024 ? `${(kb / 1024).toFixed(2)} MB/s` : `${kb.toFixed(1)} KB/s`;
  }
</script>

<div class="w-full space-y-5">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-[var(--foreground)] tracking-tight flex items-center gap-2">
        <Activity class="w-5 h-5 text-[var(--primary)]" />
        {lang === 'id' ? 'Pemantauan Infrastruktur' : 'Infrastructure Monitoring'}
      </h1>
      <p class="text-xs text-[var(--muted-foreground)] mt-0.5">
        {lang === 'id'
          ? 'Telemetri host real-time melalui WebSocket, kartu indikator risiko (Hijau/Kuning/Merah), dan log historis SQLite.'
          : 'Real-time host telemetry over WebSocket, traffic-light risk cards, and SQLite historical incident logs.'}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <span
        class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium {wsLive ? 'bg-[var(--success-subtle)] text-[var(--success)]' : 'bg-[var(--danger-subtle)] text-[var(--danger)]'}"
      >
        <span class="w-1.5 h-1.5 rounded-full {wsLive ? 'bg-[var(--success)]' : 'bg-[var(--danger)]'}"></span>
        {wsLive ? 'Live Stream' : 'Disconnected'}
      </span>

      <div class="inline-flex bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-md)] p-0.5">
        {#each ranges as r}
          <button
            type="button"
            onclick={() => (range = r)}
            class="px-2.5 py-1 text-xs rounded-[var(--radius-sm)] font-mono transition-colors {range === r ? 'bg-[var(--primary)] text-[var(--primary-foreground)] font-medium' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'}"
          >
            {r}
          </button>
        {/each}
      </div>

      <a
        href="/metrics"
        target="_blank"
        rel="noreferrer"
        class="px-2.5 py-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] text-xs text-[var(--foreground)] hover:bg-[var(--surface-elevated)] inline-flex items-center gap-1.5"
      >
        Prometheus <ExternalLink class="w-3 h-3" />
      </a>
    </div>
  </div>

  {#if errorMsg}
    <div class="flex items-center gap-2 p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/40 text-[var(--danger)] text-xs">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span>{errorMsg}</span>
    </div>
  {/if}

  <!-- Traffic-Light Risk KPI Strip (Readable from a distance: Green / Yellow / Red) -->
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
    <div class="p-4 rounded-[var(--radius-md)] border transition-colors {riskCardClasses(cpuRisk)}">
      <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        <span class="font-semibold flex items-center gap-1.5"><Cpu class="w-3.5 h-3.5" /> CPU</span>
        <span class="px-1.5 py-0.5 rounded text-[10px] font-bold {riskBadgeClasses(cpuRisk)}">
          {riskLabel(cpuRisk)}
        </span>
      </div>
      <div class="text-2xl font-bold text-[var(--foreground)] tabular-nums mt-2">
        {fmtPct(latest?.cpu_percent)}
      </div>
      <div class="text-[11px] text-[var(--muted-foreground)] mt-1">
        Warn >= 70% · Crit >= 85%
      </div>
    </div>

    <div class="p-4 rounded-[var(--radius-md)] border transition-colors {riskCardClasses(memRisk)}">
      <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        <span class="font-semibold flex items-center gap-1.5"><MemoryStick class="w-3.5 h-3.5" /> Memory</span>
        <span class="px-1.5 py-0.5 rounded text-[10px] font-bold {riskBadgeClasses(memRisk)}">
          {riskLabel(memRisk)}
        </span>
      </div>
      <div class="text-2xl font-bold text-[var(--foreground)] tabular-nums mt-2">
        {fmtPct(latest?.mem_percent)}
      </div>
      <div class="text-[11px] text-[var(--muted-foreground)] mt-1">
        Warn >= 75% · Crit >= 90%
      </div>
    </div>

    <div class="p-4 rounded-[var(--radius-md)] border transition-colors {riskCardClasses(diskRisk)}">
      <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        <span class="font-semibold flex items-center gap-1.5"><HardDrive class="w-3.5 h-3.5" /> Disk</span>
        <span class="px-1.5 py-0.5 rounded text-[10px] font-bold {riskBadgeClasses(diskRisk)}">
          {riskLabel(diskRisk)}
        </span>
      </div>
      <div class="flex items-baseline gap-2.5 mt-2 tabular-nums">
        <span class="text-2xl font-bold text-[var(--foreground)]">
          {fmtPct(latest?.disk_active_percent ?? latest?.disk_percent)}
        </span>
        <span class="text-xs font-mono text-[var(--muted-foreground)]">
          Space {fmtPct(latest?.disk_percent)}
        </span>
      </div>
      <div class="text-[11px] font-mono text-[var(--muted-foreground)] tabular-nums mt-1">
        R {formatRate(latest?.disk_read_bps ?? null)} · W {formatRate(latest?.disk_write_bps ?? null)}
      </div>
    </div>

    <div class="p-4 rounded-[var(--radius-md)] border transition-colors {riskCardClasses(netRisk)}">
      <div class="flex items-center justify-between text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        <span class="font-semibold flex items-center gap-1.5"><ArrowDownUp class="w-3.5 h-3.5" /> Network</span>
        <span class="px-1.5 py-0.5 rounded text-[10px] font-bold {riskBadgeClasses(netRisk)}">
          {riskLabel(netRisk)}
        </span>
      </div>
      <div class="text-base font-bold font-mono text-[var(--foreground)] tabular-nums mt-2.5">
        ↓ {formatRate(latest?.net_rx_bps ?? null)} · ↑ {formatRate(latest?.net_tx_bps ?? null)}
      </div>
      <div class="text-[11px] text-[var(--muted-foreground)] mt-1">
        {wsLive ? 'Telemetry link online' : 'Stream disconnected'}
      </div>
    </div>
  </div>

  <!-- Full-Width Edge-to-Edge Charts -->
  <div class="w-full p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-semibold text-[var(--foreground)]">CPU & Memory Utilization (%)</h2>
      <span class="text-[11px] font-mono text-[var(--muted-foreground)] tabular-nums">
        {points.length} samples
      </span>
    </div>
    {#if loading && points.length === 0}
      <Skeleton class="h-[260px] w-full rounded-[var(--radius-md)]" />
    {:else}
      <EChart option={cpuMemOption} height="260px" />
    {/if}
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-2">
      <h2 class="text-sm font-semibold text-[var(--foreground)]">Network Traffic (RX / TX)</h2>
      <EChart option={netOption} height="230px" />
    </div>

    <div class="p-5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)] space-y-2">
      <h2 class="text-sm font-semibold text-[var(--foreground)]">Disk Active Time & Space Used (%)</h2>
      <EChart option={diskOption} height="230px" />
    </div>
  </div>

  <!-- Server-Side Paginated Monitoring History & Incident Log -->
  <div class="w-full rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border)]">
    <div class="px-4 py-3 border-b border-[var(--border)] flex flex-wrap items-center justify-between gap-2.5">
      <div class="flex items-center gap-2">
        <History class="w-4 h-4 text-[var(--primary)]" />
        <h2 class="text-sm font-semibold text-[var(--foreground)]">
          {lang === 'id' ? 'Riwayat Detail Monitoring (Server-Side)' : 'Monitoring History Log (Server-Side)'}
        </h2>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <MultiSelect
          class="w-44"
          searchable={false}
          placeholder={lang === 'id' ? 'Filter Risiko' : 'Risk Level'}
          bind:values={logRisks}
          onchange={() => (logPage = 1)}
          options={[
            { value: 'safe', label: lang === 'id' ? 'Aman (Safe)' : 'Safe' },
            { value: 'warning', label: lang === 'id' ? 'Waspada (Warning)' : 'Warning' },
            { value: 'critical', label: lang === 'id' ? 'Kritis (Critical)' : 'Critical' },
          ]}
        />
        <DateRangePicker
          bind:from={logFrom}
          bind:to={logTo}
          onchange={() => (logPage = 1)}
        />
        <button
          type="button"
          onclick={loadServerLogs}
          disabled={logLoading}
          class="p-1.5 rounded-[var(--radius-md)] bg-[var(--surface)] border border-[var(--border-strong)] hover:bg-[var(--surface-elevated)] text-[var(--foreground)]"
          title="Refresh"
        >
          <RefreshCw class="w-3.5 h-3.5 {logLoading ? 'animate-spin' : ''}" />
        </button>
      </div>
    </div>

    <div class="overflow-x-auto">
      {#if logLoading && logItems.length === 0}
        <TableSkeleton rows={6} columns={8} />
      {:else}
      <table class="w-full text-left border-collapse text-xs">
        <thead class="bg-[var(--surface-secondary)] border-b border-[var(--border)]">
          <tr>
            <th class="px-4 py-2.5">
              <SortHeader label="Timestamp" column="recorded_at" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Status</th>
            <th class="px-4 py-2.5">
              <SortHeader label="CPU" column="cpu_percent" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Memory" column="mem_percent" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Disk" column="disk_percent" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Disk I/O" column="disk_active_percent" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Net RX" column="net_rx_bps" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5">
              <SortHeader label="Net TX" column="net_tx_bps" activeColumn={logSortBy} direction={logSortDir} onSort={handleLogSort} />
            </th>
            <th class="px-4 py-2.5 text-[11px] uppercase tracking-wider font-medium text-[var(--muted-foreground)]">Detail</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--border-subtle)] font-mono tabular-nums">
          {#if logItems.length === 0}
            <tr>
              <td colspan="9" class="px-4 py-8 text-center font-sans text-[var(--muted-foreground)]">
                {logLoading ? 'Loading…' : 'No matching telemetry records found.'}
              </td>
            </tr>
          {:else}
            {#each logItems as p}
              {@const r = rowRisk(p)}
              {@const cLevel = evalRisk(p.cpu_percent, 70, 85)}
              {@const mLevel = evalRisk(p.mem_percent, 75, 90)}
              {@const dLevel = evalRisk(p.disk_percent, 75, 90)}
              {@const ioLevel = evalRisk(p.disk_active_percent, 80, 95)}
              {@const triggers = rowTriggers(p)}
              <tr class="hover:bg-[var(--surface-elevated)]">
                <td class="px-4 py-2 text-[var(--muted-foreground)] whitespace-nowrap">
                  {new Date(p.recorded_at).toLocaleString()}
                </td>
                <td class="px-4 py-2">
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-sans font-bold {riskBadgeClasses(r)}">
                    {riskLabel(r)}
                  </span>
                </td>
                <td class="px-4 py-2 {riskTextClasses(cLevel)}">{fmtPct(p.cpu_percent, 2)}</td>
                <td class="px-4 py-2 {riskTextClasses(mLevel)}">{fmtPct(p.mem_percent, 2)}</td>
                <td class="px-4 py-2 {riskTextClasses(dLevel)}">{fmtPct(p.disk_percent, 2)}</td>
                <td class="px-4 py-2 {riskTextClasses(ioLevel)}">
                  {fmtPct(p.disk_active_percent, 1)}
                  <span class="text-[10px] font-normal text-[var(--muted-foreground)]">
                    (R {formatRate(p.disk_read_bps)} / W {formatRate(p.disk_write_bps)})
                  </span>
                </td>
                <td class="px-4 py-2 text-[var(--muted-foreground)]">↓ {formatRate(p.net_rx_bps)}</td>
                <td class="px-4 py-2 text-[var(--muted-foreground)]">↑ {formatRate(p.net_tx_bps)}</td>
                <td class="px-4 py-2 font-sans">
                  {#if triggers.length === 0}
                    <span class="text-[11px] text-[var(--success)] font-medium">
                      {lang === 'id' ? 'Semua Normal' : 'All Normal'}
                    </span>
                  {:else}
                    <div class="flex flex-wrap gap-1">
                      {#each triggers as tr}
                        <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold {tr.level === 'critical' ? 'bg-[var(--danger-subtle)] text-[var(--danger)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)]'}">
                          {tr.label}
                        </span>
                      {/each}
                    </div>
                  {/if}
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
      {/if}
    </div>

    <PaginationBar
      page={logMeta.page}
      perPage={logMeta.per_page}
      total={logMeta.total}
      totalPages={logMeta.total_pages}
      onPageChange={(p) => (logPage = p)}
      onPerPageChange={(pp) => {
        logPerPage = pp;
        logPage = 1;
      }}
    />
  </div>
</div>
