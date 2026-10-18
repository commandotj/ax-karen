# RFC 0009: Restore Disktree UI feature parity in Karen (child of 0001)

**Status:** Draft

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Define the complete Disk Tree screen and interaction contract for Karen.app,
using the preserved GPUI implementation and Disktree v0.10.1 as references.
This RFC closes the detail left open by RFC 0006: Karen must retain the actual
Disktree workflow and screen structure while rendering it in Vue 3 and Karen's
luxury steampunk visual language. It covers Explore, Review, Running and Done
states, their controls, pointer and keyboard behavior, and acceptance evidence.

## Problem

RFC 0006 correctly requires the source hierarchy, cached tree navigation,
selection, marks and guarded removal, but describes those requirements at a
high level. That left room for a new dashboard-like layout and incomplete
interaction behavior. The current Disk Tree surface still lacks parts of the
reference workflow and has had treemap tiles overlap or change stacking order
on hover. A visual resemblance alone is not parity: users need the same
navigation, scanning, inspection and safe-removal loop.

## Goals

1. Port the user-visible Disk Tree features in pinned upstream Disktree
   v0.10.1. Use Karen's archived GPUI implementation to understand the Vue
   migration, not as authority to silently add controls absent upstream.
2. Preserve RFC 0006's CLI ownership, bounded view payloads, per-session tree
   cache, absolute-path identity, explicit rescan and removal safeguards.
3. Preserve Karen's custom title bar, traffic lights and established luxury
   steampunk palette without changing Disktree's information hierarchy or
   interaction semantics.
4. Make tile geometry stable: hover, selection and mark feedback never reorder
   tiles, promote a tile, alter DOM stacking, or cover neighboring tiles.

## Non-goals

1. Changes to scan accounting, filters' core semantics, caching policy,
   deletion guards, free-space attribution, or CLI protocol ownership; RFC
   0006 owns those decisions.
2. Runtime CPU, memory or other resource-monitor tabs; RFCs 0003, 0007 and
   0008 own those screens.
3. New disk-cleaning features absent from the reference, or a new page/card
   dashboard that replaces the treemap workflow.
4. Reintroducing Disktree as a separate product, executable or app identity.

## Design

### Reference and ownership

The product baseline is upstream Disktree tag `v0.10.1`, commit
`747cb77766b5254827e09782f083bcf42fa07249`. Relevant source: `README.md`,
`crates/disktree-app/src/views.rs`, `state.rs`, `treemap_view.rs`, and
`crates/disktree-core/src/treemap.rs` and `removal.rs`. The pinned README is
[here](https://github.com/tobi/disktree/blob/v0.10.1/README.md). Karen's
`migration/gpui-reference/` is the porting reference; its extra scan-efficiency
control and right-click reveal are Karen-specific extensions, absent from this
upstream version, and are not requirements of this parity RFC. RFC 0006 remains
authoritative for scan/cache/protocol and deletion safety. Where this RFC is
more specific about screen behavior, it adds requirements to RFC 0006 without
changing its core semantics.

CLI/core owns the scan tree, paths, scan progress, marks, selection identity,
history, review plan and removal state. Vue renders bounded snapshots and
sends user intent through the existing stream protocol. Vue must not duplicate
filesystem work, derive destructive plans, or address a tile by its current
array index. Actions identify absolute paths; visual tree position is only a
layout result.

### Explore screen composition

The Disk Tree tab is a working disk-space explorer, not a landing page. Its
content order is:

1. **Top bar.** Show Karen identity, absolute breadcrumb trail, then
   back/forward history, Size / Files / Age view choices, Hidden files,
   Apparent size and depth stepper. Do not add the archived Karen-only
   scan-efficiency control to claim Disktree parity. Breadcrumbs remain visible at
   every tree depth. Long trails collapse middle segments while retaining root
   and current directory. Each in-tree crumb opens that ancestor from the
   cached tree. Its sibling picker lists sibling directories by selected metric
   with size/share information and direct-jump behavior. Choosing an ancestor
   above the current scan root widens the scan root per existing source
   behavior; it does not reinterpret a relative crumb.
2. **Totals and legend.** Show scanned bytes, file and directory counts,
   unreadable-entry count when nonzero, active name filter, and current color
   legend. Kind mode identifies Code, Agent scratch, Toolchains, Synced, Git,
   Media, Documents, Cache and Other. Age mode replaces that legend with age
   buckets. Reclaimable hatching remains independently visible in both modes.
3. **Treemap work surface.** One canvas-like rendering surface dominates the
   available width. Squarified tile area represents the selected metric;
   nesting shows directory structure. Top-level directories have a clear
   name/header band; deeper labels clip inside their own tile. Depth control
   sets the number of visible levels. The view panel does not become a grid of
   independent cards.
4. **Resizable inspection panel.** Place selection details, Worth a look,
   marks and current-volume capacity beside the treemap. Drag its left edge to
   resize; double-click the edge restores default width. At narrow widths,
   constrain or hide the panel before squeezing the mosaic into unusable tiles.
5. **Status/key bar.** Keep scan state, current volume and the context-relevant
   keyboard hints visible without covering the mosaic.

The toolbar and custom title bar are part of Karen.app. Keep the macOS traffic
lights. Do not restore a native/default title bar or browser-native select.

### Treemap appearance and behavior

The mosaic is a single stable layout. Each tile has a fixed rectangle and
paint order for a given tree, metric, depth and viewport. Category determines
hue; nesting depth adjusts fill lightness. Age mode uses recency colors.
Reclaimable content gets a hatch overlay independent of hue. Filter matches
retain full color; non-matching tiles recede while ancestor tiles remain
readable. Unreadable entries have a warning marker. Marked paths use the marked
fill/border and marked descendants show that they are covered by the parent.

Selection uses an outline. Hover uses a separate outline and a pointer-following
tooltip containing name, size, category and reclaimability when applicable.
Tooltip stays within the window and never changes tile geometry. Hover and
selection must not change z-order, DOM order, paint order, or tile position;
there is no “bring hovered node to front” behavior. These rules are explicit
regressions guards because overlapping tiles make sizes and labels unreadable.

Filtering is incremental by name. While editing, only matching tiles keep full
color and their containing directories remain subdued. Enter applies the
filtered result; Escape clears the filter before it performs any other Explore
action. Rescanning is explicit except when the user changes scan options such
as Hidden files or Apparent size, which restart the current scan as in the
source.

### Selection panel contents

With a tile selected, show its absolute/relative path, name, size, share of the
scan root, files, last write, kind, reclaimability reason and unreadable/mark
coverage state when known. Show Git working-tree details for recognized Git
checkouts when available. “Worth a look” lists a bounded set of large or
reclaimable candidates with reason and relative-size bars; choosing one selects
and reveals that path in the treemap. Marks section shows marked paths, nested
or covered counts, sizes, remove controls, and an affordance to open Review.
Volume section shows device/mount identity, used/free/total values, projected
space after eligible marks, and the volume-switch affordance. Projection must
follow RFC 0006 attribution; a mark on another mounted volume must not inflate
the current volume's projected free space.

### Pointer and keyboard contract

All controls must return focus to the treemap after activation unless a dialog,
menu or text filter explicitly owns input. When a menu is open, its arrows,
Enter and Escape are consumed by that menu; the underlying tree must not act.
The action target is the tile under the pointer if the pointer moved most
recently, otherwise the keyboard selection. Keyboard actions that operate on a
tile adopt that pointer target as the selection first.

| Input | Required behavior |
| --- | --- |
| Click tile | Select tile; clicking the already selected directory opens it. |
| Double-click tile | Select and open directory. |
| Ctrl/Cmd-click or middle-click | Toggle mark without moving selection. |
| Right-click tile | No Disk Tree action is defined by upstream v0.10.1. Do not add one as part of parity. |
| Hover tile | Show tooltip and hover outline only; never reorder or raise tile. |
| Enter | Open selected/pointed directory at any depth. |
| Space or `x` | Toggle mark on selected/pointed tile. |
| Arrow keys or `h/j/k/l` | Move selection geometrically among tiles at current level; at an edge, move to parent. |
| Tab / Shift-Tab | Move to next / previous sibling. |
| Backspace or `u` | Go to parent directory. |
| Escape | Clear active filter first; otherwise clear selection or go up one level. |
| Scroll | Zoom toward pointer; at the source-defined threshold, descend into directory under pointer. Reverse zoom walks back out. |
| Shift-scroll | Pan the magnified view. |
| `-`, `=`, `+`, `0` | Shrink, magnify, or reset treemap view. |
| `[` / `]` | Show fewer / more tree levels. |
| Cmd/Ctrl `+`, `-`, `0` | Change interface zoom, not treemap zoom. |
| Alt-Left/Right; Cmd-[ / Cmd-] on macOS; mouse side buttons | Navigate Explore history back / forward. |
| `/` or `s` | Open incremental name filter. |
| `t` | Cycle Size, Files and Age display modes. |
| `d` / `i` | Toggle Apparent size / Hidden files and rescan current root. |
| `r` | Explicitly rescan current root. |
| `v` | Open custom volume picker; choose a volume and use cached tree if available. |
| Cmd/Ctrl-O | Choose a directory to scan. |
| `g` | Widen to whole disk; clicking an ancestor crumb above scan root also widens. |
| `p` | Toggle selection/status line. |
| `o` | Reveal action target in system file manager. |
| `c` | Open Review when at least one path is marked. |
| `?` | Open keyboard help overlay. |
| `q` | Quit app, subject to platform menu conventions. |

The help overlay must document actual bindings and platform modifiers, not a
second invented keymap. The volume picker and breadcrumb sibling picker are
custom themed popovers/dialogs with visible keyboard highlight, path and device
details; they must not use native HTML `<select>` controls.

### Review, removal and completion

Review is a separate full-screen state, not a panel hidden below the mosaic.
List every marked path with size and full path, including nested/covered and
blocked status; if presentation has a display cap, state the omitted count and
keep the actual plan complete. Explain Trash versus permanent deletion and
which backends are available. Show marked bytes, actionable bytes, nested and
blocked counts, and projected free space on the measured volume. A blocked
entry must include its reason. Provide Back, clear marks, save path list, copy
agent prompt, removal-mode selection, and commit controls.

Trash is the reversible default when supported. Permanent deletion always
opens explicit confirmation naming affected paths and total bytes. Marking is
never destructive. During removal, show progress and allow Escape to stop after
the current item where the core supports it. Done state reports actual
completed/failed items and measured free-space change, then returns to Explore
with a refreshed tree. Existing removal guards remain enforced in CLI/core;
the renderer cannot bypass them.

### Layout and implementation constraints

Port the source hierarchy and behavior into the existing Vue Disk Tree surface
and CLI stream events. Keep tile painting in one canvas/SVG rendering layer or
equivalent stable renderer; do not create independently reordered tile cards.
If SVG is used, render sibling rectangles in deterministic layout order and
render hover/selection feedback as non-layout overlays that do not participate
in hit testing. Tooltip is a separate overlay and is the only element allowed
to float above the mosaic. Clip labels to each tile. Never use z-index changes
to fix hover, selection or overlap.

Do not add a second tree model in Vue. Keep absolute paths as action identity,
bound the data sent to Vue, and reuse completed in-session volume trees per RFC
0006. Switching to a cached volume, following a breadcrumb, descending,
ascending or choosing a sibling must not issue a scan. Explicit rescan replaces
only that volume cache after success; failed/cancelled rescans preserve its
last completed tree.

## Acceptance

1. **Feature inventory:** Explore, Review, Running and Done states contain all
   controls and information named in this RFC; compare each screen against the
   archived GPUI screenshot and source. The UI retains Disktree's treemap-first
   hierarchy and Karen's custom title bar/traffic lights.
2. **No tile overlap/order drift:** At normal, large and narrow window sizes,
   labels stay clipped to their own tiles and layout rectangles do not overlap
   because of hover/selection. Repeated hover over small and large tiles never
   changes sibling order, paint order, geometry or z-index. Tooltip remains
   readable and inside the window.
3. **Navigation:** Click, repeat-click, double-click, Enter, arrows, Tab,
   breadcrumb navigation, sibling jumps, history, volume switch and ancestor
   widening all target the same absolute paths as the source behavior.
   Cached navigation and cached volume revisit issue no scan request.
4. **View controls:** Size/Files/Age, Hidden files, Apparent size, depth,
   filter, wheel zoom/descend, reverse zoom, pan and interface zoom show correct
   state and do not steal tree focus after activation.
5. **Pointer and marks:** Hover only changes outline/tooltip. Modifier-click
   marks without changing selection. Keyboard mark/open actions use the
   pointer target when pointer moved last and otherwise the selected tile.
6. **Safe review:** Marking changes no files. Review shows full paths and
   accurate bytes, nested/blocked state and volume-attributed projection.
   Trash/permanent choices, permanent confirmation, cancel, save-list,
   copy-prompt, progress, interruption and Done state preserve core guards.
7. **Protocol and ownership:** Vue receives bounded snapshots; CLI/core remains
   authority for tree, marks, review plan, removal and cache. All tile actions
   use absolute paths, not layout indexes.
8. **Evidence:** Run relevant CLI/protocol and desktop checks already defined by
   RFC 0006, then manually inspect the live Karen.app at normal and large
   window sizes. Attach screenshots covering root mosaic, nested directory,
   hover tooltip, side panel, sibling picker and Review. Run `specify validate`
   and `specify sync-check` after implementation.
