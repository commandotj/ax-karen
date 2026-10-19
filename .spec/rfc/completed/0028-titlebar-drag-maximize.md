# RFC 0028: Fix custom title bar drag, cursor and double-click maximize (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Fix the custom title bar (`.masthead`) so it behaves like a native one: the pointer shows the default arrow rather than a text cursor, a press on any non-interactive part of the bar drags the window, and a double-click toggles maximize.

## Problem

In `App.vue` the drag handler is `@mousedown.self` on the header, the brand lockup and the right-hand group. `.self` fires only when the press lands on the container itself, so pressing the logo, the `KAREN` name, the edition text or the status text does not drag. Nothing handles a double-click on the bar, so it cannot maximize; on macOS the window uses `titleBarStyle: Overlay` and the bar is the only title area. The bar text is selectable, which gives the I-beam cursor and a selection highlight on drag. The Tauri capability already grants `core:window:allow-start-dragging` and `core:window:allow-toggle-maximize`.

## Goals

- No text cursor and no text selection over the title bar; interactive controls (language select, window buttons) keep their own cursors.
- Left-button press on any non-interactive part of the bar starts a window drag.
- A double-click there toggles maximize through the existing `toggleMaximizeWindow`.

## Non-goals

- No Tauri config, capability or Rust change; the shell stays a shell (invariant 10).
- No change to window controls, the macOS traffic-light layout or the tab rail.

## Design

- Replace the three `@mousedown.self` bindings with one `mousedown` handler on `.masthead`. It ignores non-primary buttons and presses whose target is inside `button, select, label`. A press with `event.detail === 2` calls `toggleMaximizeWindow`; otherwise `startDragging`. Maximizing on the second mousedown avoids relying on `dblclick`, which a native drag started by the first press can swallow on macOS.
- In `style.css`, give `.masthead` `user-select: none` and `cursor: default`, with `cursor: pointer`/default restored on the select as it is today.
- Files: `apps/desktop/src/App.vue`, `apps/desktop/src/style.css`, `apps/desktop/src/App.test.ts`.

## Acceptance

- Test: mousedown on the header, the brand name and the status text each record one `drag`.
- Test: mousedown with `detail: 2` on the brand name records `maximize` and no extra `drag`.
- Test: mousedown on the language select, a window-control button, or with a non-primary button records nothing.
- Existing window-controls test still passes.
- Manual, in the running app: the I-beam does not appear over the bar and text cannot be selected there; dragging from the logo and from the status text moves the window; double-click maximizes and restores. Computed `cursor` and `user-select` on `.masthead` checked in a browser.
- `pnpm build`, `pnpm test`, `pnpm lint` pass.
