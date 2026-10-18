# RFC 0009: Restore Disktree UI feature parity in Karen (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Replicate the complete Disktree v0.10.1 interface and workflow inside Karen.app
as a tab named **Space**. This RFC closes the detail left open by RFC 0006:
port the full treemap explorer, scan feedback, selection/inspection UI, help,
Review, removal progress and Done screens to Vue 3. Keep the source's
information hierarchy and interaction semantics; apply Karen branding and its
approved luxury steampunk visual language. This is a tab in Karen, not a
separate Disktree app or product.

## Problem

RFC 0006 correctly requires the source hierarchy, cached tree navigation,
selection, marks and guarded removal, but describes those requirements at a
high level. That left room for a new dashboard-like layout and incomplete
interaction behavior. The current Disk Tree surface still lacks parts of the
reference workflow and has had treemap tiles overlap or change stacking order
on hover. A visual resemblance alone is not parity: users need the same
navigation, scanning, inspection and safe-removal loop.

## Goals

1. Port the complete user-visible UI and workflow in pinned upstream Disktree
   v0.10.1. Preserve Karen's existing GPUI-only extensions as additions,
   clearly distinguished from upstream behavior.
2. Rename Karen's `Disk Tree` tab to `Space`; runtime volume entries navigate
   to Space with that volume as its scan root.
3. Preserve RFC 0006's CLI ownership, bounded view payloads, per-session tree
   cache, absolute-path identity, explicit rescan and removal safeguards.
4. Preserve Karen's custom title bar, traffic lights and established luxury
   steampunk palette without changing Disktree's information hierarchy or
   interaction semantics.
5. Make tile geometry stable: hover, selection and mark feedback never reorder
   tiles, promote a tile, alter DOM stacking, or cover neighboring tiles.

## Non-goals

1. Changes to scan accounting, filters' core semantics, caching policy,
   deletion guards, free-space attribution, or CLI protocol ownership; RFC
   0006 owns those decisions.
2. Runtime CPU, memory or other resource-monitor tabs; RFCs 0003, 0007 and
   0008 own those screens.
3. New disk-cleaning features beyond upstream behavior and Karen's existing
   extensions, or a new page/card dashboard that replaces the treemap workflow.
4. Reintroducing Disktree as a separate product, executable or app identity.
5. Keeping `Disk Tree` as a tab label or adding a separate Volume tab.

## Design

### Reference and ownership

The product baseline is the entire upstream Disktree tag `v0.10.1`, commit
`747cb77766b5254827e09782f083bcf42fa07249`. Relevant source: `README.md`,
`crates/disktree-app/src/views.rs`, `state.rs`, `treemap_view.rs`, and
`crates/disktree-core/src/treemap.rs` and `removal.rs`. The pinned README is
[here](https://github.com/tobi/disktree/blob/v0.10.1/README.md). Karen's
`migration/gpui-reference/` is the porting reference; its extra scan-efficiency
`migration/gpui-reference/` is the Karen porting reference. Its Power Efficiency
control and right-click reveal are existing Karen extensions, absent from this
upstream version; preserve both alongside upstream parity. The efficiency menu
keeps its saved Miser, Balanced, Aggressive and Drain My Battery worker presets
and current worker-count indication. Right-click selects the tile and reveals
its path in the system file manager. These additions must not alter upstream
tile ordering, geometry or left-click behavior. RFC 0006 remains authoritative
for scan/cache/protocol and deletion safety. Karen's host-shell integration is
additive: the tab is named Space, runtime volume entries open that volume in
Space, and the existing custom volume chooser remains available only as a
Karen integration control. It must not replace or rearrange any Disktree UI.
Where this RFC is more specific about screen behavior, it adds requirements to
RFC 0006 without changing its core semantics.

CLI/core owns the scan tree, paths, scan progress, marks, selection identity,
history, review plan and removal state. Vue renders bounded snapshots and
sends user intent through the existing stream protocol. Vue must not duplicate
filesystem work, derive destructive plans, or address a tile by its current
array index. Actions identify absolute paths; visual tree position is only a
layout result.

### Explore screen composition

The Space tab is a working disk-space explorer, not a landing page. Its
content order is:

1. **Top bar.** Show the Karen app shell, absolute breadcrumb trail, then
   back/forward history, Size / Files / Age view choices, Hidden files,
   Apparent size and depth stepper, plus Karen's existing Power Efficiency
   control as an additive setting. It selects the saved worker preset and
   displays effective worker count; it must not replace or reorder the upstream
   controls. Breadcrumbs remain visible at every tree depth. Long trails collapse middle segments while retaining
   root and current directory. Each in-tree crumb opens that ancestor from the
   cached tree. Its sibling picker lists sibling entries largest-first by the
   active metric, with category color, relative-size bar and value; it jumps
   sideways without requiring navigation to the parent first. Choosing an
   ancestor above the current scan root widens the root; it does not
   reinterpret a relative crumb.
2. **Totals and legend.** Show scanned bytes, file and directory counts,
   unreadable-entry count when nonzero, active name filter, and current color
   legend. Kind mode identifies Code, Agent scratch, Toolchains, Synced, Git,
   Media, Documents, Cache and Other. Age mode replaces that legend with age
   buckets. Reclaimable hatching remains independently visible in both modes.
3. **Treemap work surface.** One canvas-like rendering surface dominates the
   available width. Squarified tile area represents the selected metric;
   nesting shows directory structure. Top-level directories have a clear
   name/header band; deeper labels clip inside their own tile. Depth control
   sets the number of visible levels. Keep child-count and minimum-tile limits,
   including a merged “Others” tile, so tiny entries do not turn into
   unreadable or missing areas. Descend/ascend transitions grow the entering
   region into place. The view panel does not become a grid of independent
   cards.
4. **Resizable inspection panel.** Place selection details, Worth a look,
   marks and current-volume capacity beside the treemap. Drag its left edge to
   resize; double-click the edge restores default width. Show it only when
   viewport width permits a readable mosaic (52 rem in source); `p` toggles it.
   At narrow widths, hide it before squeezing the mosaic into unusable tiles.
5. **Status/key bar.** Keep scan state, current volume and the context-relevant
   keyboard hints visible without covering the mosaic.

Keep Karen's custom title bar and macOS traffic lights. The Space content uses
the Disktree composition, but no Disktree logo, app name, separate window,
default/native title bar or browser-native select.

While the first scan has no tree yet, show a centered scan state with root path,
files/directories/bytes/unreadable counts, progress meter, scan errors when
present, and a short note that marking, zoom and free-space details become
available as results arrive. Footer reports scan progress or elapsed time and
entry count. Once a completed tree exists, render that tree instead of an empty
loading page while other view updates settle.

### Treemap appearance and behavior

The mosaic is a single stable layout. Each tile has a fixed rectangle and
paint order for a given tree, metric, depth and viewport. Category determines
hue; nesting depth adjusts fill lightness. Age mode uses five recency buckets
(this week, this month, six months, this year, older).
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

With a tile selected, show its path, name, size, share of scan, files, last
write, kind/reclaim reason, Git checkout summary, and marked/covered/unreadable
states when known. Show Open for a directory and the primary Mark for removal
action; inside a marked directory offer to unmark the ancestor. “Worth a look”
lists up to six large or reclaimable candidates, stale experiments and old
worktrees with reason and relative-size bars; selecting a
candidate navigates/reveals it in the mosaic. Marks list shows up to six paths,
sizes, nested/blocked counts and a Review action, then reports any extra count.
Keep notices and the conditional macOS Full Disk Access explanation/action.
Disk section shows device, free and total space, current used space, and a
distinct projected-removal slice/value; Review affordance appears when marks
exist. Runtime volume entries open their chosen mount in Space. Space's existing
Karen volume chooser may remain as a separate host-shell control. Projection
must follow RFC 0006 attribution; a mark on another mounted volume must not
inflate current volume's projected free space.

The pointer tooltip is a compact full inspection card: icon/type, name, path,
size, share of parent, files, directories, direct-child bytes, hidden/marked/
covered badges and the relevant keyboard actions. History-button hover previews
the destination directory. Both tooltips flip around the pointer/window edge;
neither may obscure the hovered tile more than necessary or affect layout.

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
| Right-click tile | Select tile and reveal its path in the system file manager, preserving Karen's existing extension. |
| Ctrl/Cmd-click or middle-click | Toggle mark without moving selection. |
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
| Cmd/Ctrl-O | Choose a directory to scan. |
| `g` | Widen to whole disk; clicking an ancestor crumb above scan root also widens. |
| `p` | Toggle the selection/inspection panel. |
| `o` | Reveal action target in system file manager. |
| `c` | Open Review when at least one path is marked. |
| `?` | Open keyboard help overlay. |
| `q` | Quit app, subject to platform menu conventions. |

Karen host-shell integration keeps its existing volume chooser if RFC 0006
requires it; this is not Disktree UI. Runtime volume rows are the primary
cross-volume entry point requested for Karen: selecting one switches to Space
and opens that mount root, using the RFC 0006 cache when present. Do not add a
volume tab to Space.

The help overlay must document actual bindings and platform modifiers, not a
second invented keymap. The volume picker and breadcrumb sibling picker are
custom themed popovers/dialogs with visible keyboard highlight, path and device
details; they must not use native HTML `<select>` controls.

### Review, removal and completion

Review is a separate full-screen state within Space, not a panel hidden below
the mosaic. Show aligned rows with icon, name, full path, relative-size bar,
size and Unmark action. List marked paths with nested/covered and blocked
status; source displays up to 1,200 rows and states how many additional marks
remain included in the plan. Explain Trash versus permanent deletion and
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

Port the source hierarchy and behavior into the existing Vue surface renamed
Space, using the existing CLI stream events. Keep tile painting in one
canvas/SVG rendering layer or
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

Keep global app commands alongside the tab: Open Folder, Show in Finder/File
Manager, Rescan, Back, Forward, Quit, and platform-standard hide/close actions.
Expose these through Karen's app shell/menu integration with equivalent
platform shortcuts. `?` opens a complete keyboard/mouse reference overlay.

## Acceptance

1. **Feature inventory:** Space contains complete Explore, initial scan,
   Review, Running, Done and keyboard-help states; breadcrumb/sibling and
   history previews; selection, findings, marks, notices, privacy hint, disk
   meter and all upstream controls. Karen's Power Efficiency control retains
   all available saved presets and effective worker count; right-click selects
   and reveals the tile without changing tile order or geometry. Compare each state against upstream source
   and screenshot. Space tab label and runtime volume entry behavior match this
   RFC. Karen retains custom title bar/traffic lights and owns app identity.
2. **No tile overlap/order drift:** At source minimum window size (900×600),
   normal, large and narrow content widths,
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

## Closing notes

Closed as Implemented by the maintainer's decision with these acceptance items
not met; any follow-up needs a new RFC:

* Application menu integration (Open Folder, Rescan, Back, Forward and
  platform shortcuts through the Tauri shell).
* Escape to stop a removal after the current item; the CLI has no cancel
  command for removal.
* Evidence screenshots listed in Acceptance 8 (root mosaic, nested directory,
  tooltip, side panel, sibling picker, Review).
* Explore history lives in the Vue view as absolute-path view requests, not in
  the CLI.
* Windows `reveal` is written but has never been compiled or run.
