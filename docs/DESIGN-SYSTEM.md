# MeetoOps Design System — Warm Obsidian Specification

This document defines the complete visual, typographic, layout, component, and data-visualization design system for **MeetoOps**.

---

## 1. Design Philosophy

MeetoOps is an operational workstation used by developers and DevOps engineers for hours at a time. Its design system — **Warm Obsidian** — optimizes for:

1. **Visual Calm Under Pressure**: Warm, low-glare dark surfaces reduce eye strain during incident response and long debugging sessions.
2. **Information Density Without Clutter**: Compact tables, clear typographic hierarchy, and purposeful whitespace maximize visible infrastructure state.
3. **Semantic Clarity**: Color is reserved for meaning (primary action, active state, health status, chart series) rather than decoration.
4. **Original Identity**: Distinct from Portainer's cool blue panels and Grafana's orange-on-black chrome.

---

## 2. Color System & Semantic Tokens

### 2.1 Dark Theme: Warm Obsidian (Default)

| Token | Hex | Role |
|---|---|---|
| `--primary` | `#D6A76C` | Primary CTA buttons, active nav accent, focus rings, primary chart series |
| `--primary-hover` | `#E3B77E` | Hover state for primary buttons and links |
| `--primary-foreground` | `#171614` | Text/icons rendered on `--primary` backgrounds |
| `--secondary` | `#C98262` | Secondary emphasis, secondary chart series |
| `--background` | `#171614` | Primary application canvas (Warm Obsidian) |
| `--sidebar` | `#1D1B18` | Navigation sidebar and top utility bar (Dark Espresso) |
| `--surface` | `#24221F` | Standard cards, panels, table containers, inputs (Warm Charcoal) |
| `--surface-elevated` | `#302C27` | Dropdowns, modals, tooltips, table row hover (Warm Stone) |
| `--border` | `#3A352F` | Primary panel/input/layout borders (Dark Taupe) |
| `--border-subtle` | `#2C2824` | Internal table row dividers and subtle separators |
| `--foreground` | `#F5F0E8` | Primary headings, body text, values (Warm Ivory) |
| `--muted-foreground` | `#A69D90` | Labels, column headers, timestamps, secondary metadata (Warm Gray) |
| `--subtle-foreground` | `#7A7267` | Placeholders, disabled text |
| `--success` | `#8DBB8A` | Running containers, healthy checks, online agents (Sage Green) |
| `--warning` | `#E5A85B` | High utilization warnings, paused containers, probable detection (Warm Amber) |
| `--danger` | `#D97768` | Stopped/unhealthy errors, offline agents, destructive actions (Soft Red) |
| `--info` | `#8DAEB8` | Informational notices, neutral telemetry, network RX series (Muted Blue) |

### 2.2 Semantic Status Surfaces (Tinted Badges & Alerts)

Derive translucent status backgrounds via `color-mix()` so both Dark and future Light themes remain harmonious without hardcoding extra hex codes:

```css
--success-subtle: color-mix(in srgb, var(--success) 14%, transparent);
--warning-subtle: color-mix(in srgb, var(--warning) 14%, transparent);
--danger-subtle:  color-mix(in srgb, var(--danger) 14%, transparent);
--info-subtle:    color-mix(in srgb, var(--info) 14%, transparent);
--primary-subtle: color-mix(in srgb, var(--primary) 14%, transparent);
```

### 2.3 Future Light Warm Theme Readiness

All components reference semantic tokens only. A future `[data-theme="warm-alabaster"]` theme can override the same token set (e.g., `#F7F4EE` background, `#EFEAE1` sidebar, `#FFFFFF` surface, `#221F1B` foreground, `#9C6B2F` primary) with zero component code changes.

---

## 3. Typography

### Font Families
- **UI / Sans**: `'Inter'`, system-ui, `-apple-system`, `sans-serif`
- **Code / Data / Terminal**: `'JetBrains Mono'`, `'IBM Plex Mono'`, `ui-monospace`, `monospace`

### Type Scale

| Role | Size | Weight | Line Height | Tracking | Usage |
|---|---|---|---|---|---|
| Page Title (`h1`) | `20px` (`1.25rem`) | `600` | `1.3` | `-0.015em` | Top of every workspace view |
| Section Title (`h2`) | `15px` (`0.9375rem`) | `600` | `1.35` | `-0.01em` | Panel headers, settings groups |
| Card / Sub-header (`h3`) | `13px` (`0.8125rem`) | `600` | `1.4` | `0` | Table toolbars, stat labels |
| Body / Table Cell | `13px` (`0.8125rem`) | `400` | `1.45` | `0` | Default workspace text |
| Dense Metadata / Header | `11px` (`0.6875rem`) | `500` | `1.3` | `0.04em` | Uppercase table column headers, nav group labels |
| KPI Figure | `24px` (`1.5rem`) | `600` | `1.15` | `-0.02em` | Dashboard summary stats (`tabular-nums`) |
| Monospace Data | `12px` (`0.75rem`) | `400` | `1.5` | `0` | Container IDs, IPs, ports, logs, SQL, Redis keys |

**Mandatory Rule:** Apply `font-variant-numeric: tabular-nums` to all numeric columns, KPIs, charts, timestamps, and byte/rate readouts so values never jitter during WebSocket updates.

---

## 4. Layout & Spacing System

- **Base Grid**: `4px` unit (`4, 8, 12, 16, 20, 24, 32`).
- **Sidebar**:
  - Expanded width: `240px`
  - Collapsed width: `56px` (icons + tooltips)
  - Background: `--sidebar` (`#1D1B18`), right border `1px solid var(--border)`
  - Grouped sections:
    1. **Overview**: Dashboard, Monitoring, Alerts
    2. **Docker**: Containers, Images, Volumes, Networks, Stacks
    3. **Data & Services**: Service Discovery, Database Explorer, Redis Explorer
    4. **Operations**: Log Center, Terminal, Servers
    5. **System**: Settings, Audit Logs
- **Top Context Bar**: `48px` height, `--sidebar` background, bottom border `1px solid var(--border)`. Contains: active server selector, live WebSocket connection status pill, global quick-jump search (`Ctrl+K`), and user/session menu.
- **Workspace Padding**: `20px`–`24px` outer padding, `16px` gap between major panels.
- **Responsive Breakpoints**:
  - Desktop (`>= 1280px`): Full expanded sidebar, multi-column grids.
  - Laptop (`1024px–1279px`): Collapsible sidebar, dense 2-column grids.
  - Tablet (`768px–1023px`): Auto-collapsed sidebar rail, stacked detail panes.

---

## 5. Component Conventions

### Buttons
- **Primary**: `bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary-hover)]`
- **Secondary**: `bg-[var(--surface-elevated)] text-[var(--foreground)] border border-[var(--border)] hover:border-[var(--muted-foreground)]`
- **Ghost**: `text-[var(--muted-foreground)] hover:bg-[var(--surface-elevated)] hover:text-[var(--foreground)]`
- **Destructive**: `bg-[var(--danger-subtle)] text-[var(--danger)] border border-[color-mix(in_srgb,var(--danger)_35%,transparent)] hover:bg-[var(--danger)] hover:text-[#171614]`
- **States**: Every interactive button must implement `hover`, `focus-visible` (`2px solid var(--ring)`), `disabled` (`opacity-50 pointer-events-none`), and `loading` (inline spinner + disabled).

### Cards & Panels
- Background `--surface`, border `1px solid var(--border)`, border-radius `6px` (`--radius-md`).
- No drop shadows on standard flat panels; reserve subtle shadow (`0 8px 24px rgba(0,0,0,0.45)`) exclusively for floating modals, popovers, and command palettes.
- Never nest cards inside cards; use subtle dividers (`--border-subtle`) or horizontal tabs inside a panel.

### Data Tables (TanStack Table + TanStack Virtual)
- Row height: `38px` (compact operational density).
- Header row: `34px`, sticky, `11px` uppercase `--muted-foreground`, `--sidebar` background, `1px solid var(--border)` bottom border.
- Row separators: `1px solid var(--border-subtle)`.
- Row hover: `--surface-elevated`.
- Explicit empty, loading skeleton, and error states inside the table frame.

### Badges & Status Indicators
- Use small `6px` status dots paired with concise text (never color alone — supports accessibility/color-blind users):
  - `Running` / `Online` / `Healthy` / `Confirmed` -> `--success`
  - `Paused` / `Degraded` / `Restarting` / `Probable` -> `--warning`
  - `Exited` / `Unhealthy` / `Offline` / `Critical` -> `--danger`
  - `Created` / `Info` -> `--info`
  - `Unknown` / `Unavailable` -> `--muted-foreground`

### Secret Masking Field
- Secret environment variables, API tokens, and credentials render as `••••••••••••` in monospace with a gated **Reveal** button (only shown when RBAC permits; triggers audited reveal and auto-hides after 30 seconds).

---

## 6. Data Visualization (Apache ECharts)

### Chart Palette Order
1. `#D6A76C` (Muted Amber — CPU / Primary series)
2. `#8DAEB8` (Muted Blue — Memory / RX traffic)
3. `#C98262` (Terracotta — Disk I/O / TX traffic)
4. `#8DBB8A` (Sage Green — Cache hits / Healthy count)
5. `#B59CC2` (Muted Mauve — Auxiliary 5th series only when needed)

### Chart Rules
- Background: transparent (inherits `--surface`).
- Gridlines: dashed `#2C2824` (`--border-subtle`).
- Axis labels: `11px`, `#A69D90` (`--muted-foreground`).
- Tooltips: `#302C27` background, `1px solid #3A352F`, `#F5F0E8` text, crisp monospace values.
- Area fills: very subtle (`8%–12%` opacity flat or gentle fade), never neon glow.
- Missing telemetry gaps: render as broken line (`connectNulls: false`), never interpolated as `0`.

---

## 7. Terminal & Code Editor Styling

### Terminal (`xterm.js`)
- Background: `#1A1816`
- Foreground: `#F5F0E8`
- Cursor: `#D6A76C`
- Selection: `rgba(214, 167, 108, 0.25)`
- ANSI Colors: Warm-calibrated high-contrast palette (Green `#8DBB8A`, Red `#D97768`, Yellow `#E5A85B`, Blue `#8DAEB8`, Magenta `#C29BB8`, Cyan `#7FB2AC`).

### Code & Query Editor (`Monaco Editor`)
- Custom theme `meetoops-warm-obsidian` registered at boot:
  - Editor background: `#1A1816`
  - Line highlight: `#24221F`
  - Line numbers: `#7A7267`
  - Keywords: `#D6A76C`
  - Strings: `#8DBB8A`
  - Numbers: `#C98262`
  - Types/Identifiers: `#8DAEB8`
  - Comments: `#7A7267`

---

## 8. First-Run Setup Wizard UX

- Centered focused card (`max-width: 640px`) on `--background` (`#171614`).
- Clear horizontal 5-step progress stepper (`Welcome` -> `Admin` -> `Preferences` -> `Docker` -> `Complete`).
- Warm Obsidian branding mark, clean form validation, live password strength meter, and real-time Docker connection test feedback.
- Zero terminal commands required for local Docker; clean copyable one-liner for optional remote agent registration.
