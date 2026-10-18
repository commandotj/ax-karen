# RFC 0012: Add top-bar system tray icon and menu (child of 0001)

**Status:** Draft

**Parent:** [0001](completed/0001-karen-system-resource-app.md)

## Summary

Add a persistent top-bar system tray (macOS menu bar / system notification
area) icon and menu to the Karen Tauri 2 desktop shell so users can glance at
Karen, summon or hide the main window, and jump directly to any monitor tab from
the top bar without violating Karen's CLI-owned architecture.

## Problem

Karen currently runs only as a standard application window. When the window is
occluded, minimized or closed during continuous monitoring, users have no
top-bar menu bar presence to check status, bring Karen back to the foreground
or jump straight to a specific instrument view (`Runtime`, `CPU`, `Memory`,
`Battery`, `Network`, `Space`).

## Goals

1. Register a native system tray / macOS top-bar icon in
   `apps/desktop/src-tauri/` using Tauri 2's `tray-icon` capability.
2. Provide direct window toggle and tab-navigation actions from the tray icon
   and its menu (`Show / Hide Karen`, `Runtime`, `CPU`, `Memory`, `Battery`,
   `Network`, `Space`, and `Quit Karen`).
3. Allow the Vue frontend to forward a concise summary status line/tooltip
   derived from the `karen --stream-json` CLI events to the tray without adding
   any monitor crate dependencies or sampling logic to Tauri (preserving
   Invariant 10).
4. Ensure the tray icon asset renders cleanly at top-bar sizes (`16px`–`22px`
   `@1x`/`@2x`) on macOS, Linux and Windows.

## Non-goals

1. Adding `monitor-cpu`, `monitor-memory`, `monitor-disk`, `monitor-battery` or
   `monitor-network` dependencies to `apps/desktop/src-tauri/`.
2. Spawning a second independent `karen` CLI process for the tray.
3. Implementing a separate multi-window webview popover backend unless needed in
   a future RFC.

## Design

### Tauri host shell (`apps/desktop/src-tauri/`)

1. Enable the `tray-icon` (and `image-png`) features on `tauri` in
   `apps/desktop/src-tauri/Cargo.toml`.
2. In `apps/desktop/src-tauri/src/main.rs`, configure `TrayIconBuilder` during
   `.setup(...)`:
   - Load the dedicated tray icon from `apps/desktop/src-tauri/icons/` (aligned
     with RFC 0013's icon export).
   - Build a native `Menu` with items:
     -Status header item (disabled/read-only label, e.g., `Karen — System Monitor`,
      updated by Vue when CLI samples arrive)
     - Separator
     - `Show / Hide Window`
     - Separator
     - `Open Runtime`
     - `Open CPU`
     - `Open Memory`
     - `Open Battery`
     - `Open Network`
     - `Open Space`
     - Separator
     - `Quit Karen`
   - On tray menu selection of a tab action, unminimize/show and focus the
     `main` window and emit a `tray-navigate` event with the target tab id so
     `apps/desktop/src/App.vue` switches tabs without starting an unintended
     disk scan (unless `Space` is explicitly chosen).
   - Expose a lightweight Tauri command `update_tray_status(summary: String)`
     (or use `@tauri-apps/api/tray`) so `App.vue` can push the latest CLI-derived
     summary (such as `CPU 14% · MEM 62%`) into the tray tooltip and status
     menu row.

### Frontend integration (`apps/desktop/src/App.vue`)

Listen for `tray-navigate` events when running inside Tauri to activate the
requested tab, and push concise status updates whenever a `resources` event
arrives from `KarenCli`.

## Acceptance

1. Launching Karen displays the Karen icon in the macOS top menu bar / system
   tray.
2. Clicking tray menu actions shows/hides the main window, switches directly to
   the selected tab (`Runtime`, `CPU`, `Memory`, `Battery`, `Network`, `Space`),
   and quits cleanly when `Quit Karen` is chosen.
3. Tray tooltip/status reflects live CLI telemetry pushed from Vue, and
   `apps/desktop/src-tauri/Cargo.toml` retains zero dependencies on any
   `monitor-*` crates.
4. `cargo xtask lint`, `pnpm build` and desktop verification pass.
