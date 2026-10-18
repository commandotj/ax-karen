# RFC 0019: Modularize desktop views to reduce App.vue size

**Status:** Draft

## Summary

Extract self-contained view components (`RuntimeView.vue`, `CpuView.vue`, `MemoryView.vue`, `BatteryView.vue`, `NetworkView.vue`, `SpaceView.vue`) and shared presentation types/utilities from `App.vue`. Keep `App.vue` strictly as the root shell orchestrator (CLI lifecycle, window chrome, and routing). Maintain 100% test coverage and preserve all steampunk styling and invariants.

## Problem

`apps/desktop/src/App.vue` has grown to over 1,280 lines. It currently bundles:
1. Root application orchestration (window drag, minimization, CLI child process management, system tray sync).
2. Telemetry and state caches for 6 different tabs (Runtime, CPU, Memory, Battery, Network, Space/Disk).
3. The complete markup and local handlers for every individual view (such as the entire Disk Tree inspector, confirmation modals, crumb sibling menus, process ledgers, and telemetry sparkline paths).

This monolithic structure reduces readability, complicates isolated testing of individual views, and makes maintenance error-prone.

## Goals

1. **Decompose `App.vue`**: Reduce `App.vue` from ~1,280 lines down to a clean, focused shell orchestrator (< 350 lines).
2. **Dedicated Component Organization**:
   - Organize all sub-components and view panels under `apps/desktop/src/components/`:
     - `components/TabWheel.vue`: Rotary drum tab wheel selector.
     - `components/views/RuntimeView.vue`: Processor load, physical memory, battery/network mini-cards, and volume meter instruments.
     - `components/views/CpuView.vue`: CPU process load table and system load header.
     - `components/views/MemoryView.vue`: Physical memory console, memory pressure chart, and resident memory table.
     - `components/views/BatteryView.vue`: Battery state/reserve console, charge trace chart, and electrical/cell fact grid.
     - `components/views/NetworkView.vue`: Network throughput console, dual-channel trace chart, and sorted interface table.
     - `components/views/SpaceView.vue`: Disk treemap canvas, toolbar, breadcrumbs, history, filters, inspector panel, and review/removal flow.
3. **Pure Presentation Types & Helpers**:
   - Centralize resource and telemetry types in `types.ts` to eliminate duplicate type declarations.
   - Move shared formatting helpers (`formatBytes`, `formatRate`, `formatDuration`, `chartPath`) into a pure utility module (`format.ts`).
4. **Preserve Invariants & Quality**:
   - Zero regression in visual styling (steampunk brass/enamel layout remains pixel-perfect).
   - Maintain 100% line and statement test coverage across all new and existing components.

## Non-goals

- Altering the CLI protocol or backend data contracts (`karen --stream-json`).
- Changing the visual appearance, color palette, or user-visible behaviors.
- Introducing a state management library (Pinia, Vuex); Vue 3 props and emits/standard composables are sufficient.

## Design

### Architecture

```
apps/desktop/src/
├── App.vue               (Root shell: window controls, masthead, TabWheel, CLI connection, route view)
├── format.ts             (Pure formatters: bytes, rates, durations, SVG chart paths)
├── types.ts              (Shared telemetry and disk models)
└── components/
    ├── TabWheel.vue      (Rotary drum tab selector)
    ├── tabWheel.ts       (Pure tab wheel math & tray formatters)
    └── views/
        ├── RuntimeView.vue
        ├── CpuView.vue
        ├── MemoryView.vue
        ├── BatteryView.vue
        ├── NetworkView.vue
        └── SpaceView.vue
```

### Component Contracts

- All view components are strictly controlled by props and communicate user actions (such as `open-disk`, `select-volume`, `navigate-tab`, `scan`, `mark`, `toggle-power`) via typed events.
- Root keyboard shortcuts (e.g. `g`, `p`, `u`, `x`, `?` in Space view) are handled or cleanly delegated.
- The Canvas element in `SpaceView.vue` preserves its resize observer and paint lifecycle.

## Acceptance

1. `App.vue` size is reduced to < 350 lines.
2. All components and views are cleanly organized under `apps/desktop/src/components/`.
3. Vitest suite passes with 100% line and statement coverage (`pnpm --dir apps/desktop exec vitest run --coverage`).
4. Type check and production build succeed cleanly (`pnpm build`).
5. RFC lifecycle checks pass (`specify.mjs validate` and `specify.mjs sync-check`).
