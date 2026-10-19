# RFC 0027: Clarify Disk Tree click and context-menu actions (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Make Disk Tree pointer actions unambiguous (GitHub issue #14). A single click only selects a tile. A double-click drills into a directory and does nothing on a file. A right-click opens a context menu offering mark/unmark and reveal in Finder, instead of revealing immediately.

## Problem

`selectTile` in `DiskTreeView.vue` opens a directory when it is already selected, so a second slow click navigates. Even with `open: false`, the CLI answers a request for a nested tile with that tile's parent as the view, so a single click on a nested tile moves the view and reads as drilling in. `contextTree` selects the tile and reveals it in the file manager at once, with no way to mark from the pointer other than Ctrl/Cmd-click or middle-click, which users cannot discover.

## Goals

- Single click selects only; it never navigates.
- Double-click on a directory drills in; on a file it does not navigate.
- Right-click selects the tile and shows a menu with Mark for removal / Unmark and Reveal in Finder. Opening the menu removes nothing.
- Menu actions reuse the existing `toggleMark` and `revealPath` events, so CLI guards apply unchanged: `toggle_mark` rejects the root, missing paths and paths covered by a marked ancestor; `reveal` accepts only absolute paths without `..` under a cached or scanning root.

## Non-goals

- No new CLI command, Tauri capability or Rust change.
- Keyboard behavior (Enter, Space, X, O) and Cmd-click or middle-click marking are unchanged. On macOS, Ctrl-click is delivered as `contextmenu` and never as `click`, so it opens the new menu; the existing Ctrl-click mark branch was already unreachable there.
- No change to removal review, confirmation or guards.

## Design

Changes are in `apps/desktop/src/views/DiskTreeView.vue`, with tests in `apps/desktop/src/App.test.ts` and `DiskTreeView.test.ts`.

Clicks:

- Select-only requests (single click, right-click) are sent with `keepView`. `App.vue` records their request ids and, for the matching `tree_view` reply, applies only the selection and marks; tiles, breadcrumbs, siblings, findings and totals stay as they were. No CLI or protocol change.

- `selectTile` already uses `event.detail`; drop the `selectedBefore` shortcut and open only when `tile.is_dir && event.detail === 2`. `=== 2` keeps a third rapid click from opening whatever tile now sits under the pointer after the drill.
- `auxclick` is wired to `clickTreeCanvas`; ignore it unless `button === 1` so a right-click cannot also run `selectTile`.

Menu:

- `contextTree` selects the tile without opening, then stores the tile path and pointer position. It no longer emits `revealPath`.
- The menu is a `position: fixed` overlay outside the canvas, clamped to the window with the same flip logic as `showTileTooltip`. The one-Canvas, hover-stroke-only and z-order invariants for tiles are untouched.
- Items: mark or unmark (emits `toggleMark`) and reveal (emits `revealPath`). The mark label is computed from `props.marks` and the stored path, not a snapshot taken at open time. A path covered by a marked ancestor shows a disabled "Inside marked X" item (reusing the `coveredMarkName` logic for the menu path) instead of sending a request the CLI would reject.
- The reveal label reads "Reveal in Finder" on macOS and "Reveal in file manager" elsewhere (`isMacOS`), because the CLI uses `open -R`, Explorer or `xdg-open` per platform.
- Mount with `Teleport to="body"` and `position: fixed` like `.volume-options`; reuse the `.crumb-menu` / `.power-menu` styles with no new colors or fonts. The menu handles its own `contextmenu` with `prevent` so the WebView default menu does not appear.
- Hide the hover tooltip while the menu is open (stroke-only hover rule is unaffected).
- Keyboard-invoked `contextmenu` (ContextMenu key, Shift+F10) has no usable coordinates; fall back to the selected tile, positioned at its rectangle center.

Closing:

- Close after an action, on Escape, on outside pointerdown, on canvas wheel, and on window `resize` and `blur`.
- Extend `closeVolumeMenu` so a pointerdown inside `.tree-context-menu` does not close it, otherwise the item click never fires.
- Escape closes the menu first and returns before the `cancelReview` branch in `closeVolumeMenuOnEscape`. `handleDiskKeyboard` also guards on the open menu (as it does for `helpOpen`), because its Escape branch otherwise runs `goToParent()`. While the menu is open, other tile shortcuts do nothing.
- `contextTree` itself requests the tree, so the response replaces `props.tiles` while the menu is open. Do not close on any tile change; close only when the menu path is no longer in `tiles` or the view directory changes.
- Close when `reviewScreen` or `removalConfirm` turns on, since the canvas is unmounted behind `v-if`.

Accessibility:

- `role="menu"` with an `aria-label`, items as `role="menuitem"` buttons, focus moves to the first item on open, ArrowUp/ArrowDown/Home/End move focus, Tab closes, and focus returns to the canvas on close.
- Update the canvas `aria-label` to mention the context menu.

Copy:

- Menu labels are locale keys, added to all nine locales (de, en, es, fr, ja, ko, ru, zh-CN, zh-TW) and the `LocaleMessageSchema` type, since RFC 0022 puts DiskTreeView in scope and neighbouring buttons already use `t()`. The remaining English literals in the view are existing debt and out of scope.
- Update the two help rows for click and right-click and the keybar/aria text that mention old behavior.

## Acceptance

Tests (in `App.test.ts` unless noted):

- Update "selects on click, opens a selected or double-clicked directory": a single click on an already-selected directory now requests `open: false`. Rename it.
- Update the right-click reveal test: right-click shows the menu and sends no `reveal`; choosing "Reveal in Finder" sends `reveal` with the absolute path.
- A double-click sends two requests, `open: false` then `open: true`; the test asserts the final one opens.
- A select-only reply whose breadcrumbs point at the clicked node's parent leaves the visible breadcrumbs unchanged, while the double-click reply moves them.
- A click with `detail: 2` on a directory requests `open: true`; on a file `open: false`; `detail: 3` does not open again.
- `auxclick` with `button: 2` does not run `selectTile`; `button: 1` still toggles the mark.
- contextmenu on a tile selects it, shows the menu and sends neither `toggle_mark` nor `reveal`.
- The menu item toggles the mark with the absolute path; its label flips to Unmark for a marked path.
- The menu survives the `tree_view` response its own `requestTree` triggers, and closes when its path leaves `tiles` or the directory changes.
- Escape closes the menu without cancelling a review and without calling `goToParent()` (no `tree_view` sent); outside pointerdown (dispatched on a real element) closes it; pointerdown inside does not.
- Entering the review screen closes the menu.
- A path covered by a marked ancestor shows the disabled "Inside marked X" item and sends no `toggle_mark`.
- Keyboard `contextmenu` with no hit falls back to the selected tile.
- Locale test: every locale defines the new keys.
- Tests mock `getBoundingClientRect` and use tiles with coordinates, since jsdom returns zero-size rects and `tileAt` would otherwise never hit.

Build and manual:

- `pnpm build`, `pnpm test`, `pnpm lint` pass; the desktop UI checked manually at practical window sizes.
