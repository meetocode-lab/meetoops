# DESIGN.md — MeetoOps Visual Identity (Blue Aurora)

MeetoOps adopts the **Blue Aurora** visual identity (aligned with `meetofast`), featuring instant **Light / Dark / System** theme switching persisted in `localStorage` (`meetoops-theme-preference`) and hydrated before first paint to prevent theme flash.

---

## 1. Visual Direction & Theme Policy

- **Theme Modes**: **Light** (`[data-theme="light"]`), **Dark** (`[data-theme="dark"]`), and **System** (follows OS `prefers-color-scheme` live).
- **Brand Palette**: Cobalt Blue (`#2563eb` light / `#3b82f6` dark), Cyan (`#22d3ee`), Sky (`#7dd3fc`), Crisp White (`#f8fbff` / `#ffffff`) in Light Mode, and Deep Navy Aurora (`#071426` / `#0d1d33`) in Dark Mode.
- **Strict Exclusions**: Never use purple, violet, magenta, or pink in brand accents, illustrations, or gradients. No decorative glassmorphism or emoji icons in UI chrome.

---

## 2. Semantic Design Tokens (`apps/web/src/app.css`)

### Light Theme (`:root`, `[data-theme="light"]`)
- `--primary`: `#2563eb` | `--primary-hover`: `#1d4ed8` | `--primary-subtle`: `#eff6ff`
- `--background`: `#f8fbff` | `--sidebar`: `#ffffff` | `--surface`: `#ffffff` | `--surface-elevated`: `#f1f7ff`
- `--border`: `#dbeafe` | `--border-strong`: `#bfdbfe` | `--border-subtle`: `#eff6ff`
- `--foreground`: `#0f172a` | `--muted-foreground`: `#475569` | `--subtle-foreground`: `#64748b`
- `--success`: `#15803d` | `--warning`: `#b45309` | `--danger`: `#b91c1c` | `--info`: `#0369a1`

### Dark Theme (`[data-theme="dark"]`)
- `--primary`: `#3b82f6` | `--primary-hover`: `#60a5fa` | `--primary-subtle`: `#162c4a`
- `--background`: `#071426` | `--sidebar`: `#0b192e` | `--surface`: `#0d1d33` | `--surface-elevated`: `#193252`
- `--border`: `#1e3552` | `--border-strong`: `#2c4b70` | `--border-subtle`: `#132742`
- `--foreground`: `#f8fafc` | `--muted-foreground`: `#cbd5e1` | `--subtle-foreground`: `#94a3b8`
- `--success`: `#4ade80` | `--warning`: `#fbbf24` | `--danger`: `#f87171` | `--info`: `#38bdf8`

---

## 3. Typography & Operational Precision

- **UI Sans**: `Inter`, system-ui, sans-serif
- **Code / Telemetry / Terminal**: `JetBrains Mono`, ui-monospace, monospace
- **Tabular Numerals**: Mandatory `tabular-nums` on all metrics, percentages, ports, byte sizes, and timestamps.
- **Missing Data**: Always display `—` or `Unavailable` for `null` telemetry, never `0`.

---

## 4. Mandatory UI Primitives & Layout Rules

- **No Native Form Chrome**: Never use raw `<select>`, `<input type="checkbox">`, or `<input type="date">` in views. Always use:
  - `apps/web/src/components/ui/Select.svelte` — shadcn-style popover select with **search enabled by default** (`searchable = true`; pass `searchable={false}` for small static lists).
  - `apps/web/src/components/ui/MultiSelect.svelte` — shadcn-style searchable multi-select with checkbox items and selection count badge.
  - `apps/web/src/components/ui/Checkbox.svelte` — custom Blue Aurora checkbox with crisp focus ring and check icon.
  - `apps/web/src/components/ui/DatePicker.svelte` & `DateRangePicker.svelte` — custom Blue Aurora calendar popovers.
  - `apps/web/src/components/ui/PaginationBar.svelte` & `SortHeader.svelte` — mandatory server-side pagination (with page-size selector `10/25/50/100`) and per-column sorting for all tables/lists.
  - `apps/web/src/components/ui/Skeleton.svelte` & `TableSkeleton.svelte` — mandatory skeleton loaders for all page, card, chart, and table loading states (no plain `"Loading…"` text).
- **Independent Scroll Layout**: Root workspace uses `h-screen overflow-hidden`. Top Navbar stays fixed, Sidebar `<nav>` scrolls independently, and `<main>` workspace scrolls independently.
- **Full-Width Panels**: Operational views inside `<main>` stretch `w-full` without artificial horizontal `max-w-*` gutters.
