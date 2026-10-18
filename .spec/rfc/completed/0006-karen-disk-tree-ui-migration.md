# RFC 0006: Migrate the complete Karen interface and interactions to Vue 3 (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Complete the Vue 3 migration of Karen's disk space management interface and
interaction model. Match the source app's screen structure and navigation:
breadcrumb trail with sibling jumps, volume and scan controls, scan totals,
treemap, selection details, marks, review, and guarded removal confirmation.
Keep Karen's established visual identity around that structure. Directory
navigation and returning through breadcrumbs must use the cached scan tree,
not start another filesystem scan.

## Problem

RFC 0004 requires preserving Disk Tree navigation, selection, marks,
review, removal and scan controls in Karen.app. The current Vue screen only
renders a simplified treemap. Tile clicks start new scans, the screen has no
breadcrumb trail, selection panel, sibling jumps, marks or review flow, and
switching volumes can discard the visible tree without loading the selected
volume. This is an incomplete migration of the already-approved product
behavior.

## Goals

Reproduce the source screen hierarchy and all existing disk
workflow behavior in Vue 3. Preserve treemap proportions, colors and labels;
provide clickable breadcrumbs and sibling navigation; show selected-node
details and scan totals; preserve mark, review, confirmation, removal guards,
volume free-space projections and scan controls. Cache completed trees in the
CLI for the app session so descending, ascending, sibling jumps and returning
to a previously scanned volume do not repeat the scan. Provide an explicit
rescan action to refresh one volume's cached tree.

## Non-goals

Changes to file-size accounting, scan filtering semantics, deletion policy,
resource telemetry, process monitoring, persistent cache across app restarts,
new disk-management capabilities, or a redesign of the established Karen
brand and steampunk visual identity.

## Design

Use the preserved GPUI implementation in `migration/gpui-reference/` as the
behavioral reference. Use its README
screen description as the layout reference. Keep the full `Node` tree, marks,
selection, scan root and review plan in the CLI/core. Extend the CLI JSON-line
protocol with bounded requests and events for cached tree navigation,
breadcrumbs, sibling choices, selection, marks, review and removal
confirmation. Keep Vue as a renderer and input surface; do not move filesystem
operations, tree ownership or deletion policy into Tauri or Vue.

The CLI retains completed per-volume trees for the lifetime of the app process.
Navigating within a cached tree resolves paths against that tree and returns
only the current bounded treemap tiles, breadcrumbs, sibling list, selection
details, totals and marks summary. A cache miss starts a scan. Explicit rescan
replaces only that volume's cached tree after successful completion; failed or
cancelled scans leave the prior completed tree available. Tile and breadcrumb
actions address absolute paths so tree layout sorting cannot redirect a
selection. Vue reproduces the source screen's hierarchy: controls and trail at
top, totals/filter/legend above the mosaic, mosaic as the main work surface,
and selection/mark/volume information in its side panel, followed by a
separate review and confirmation flow.

## Acceptance

Compare each Disk Tree state and interaction against the preserved source:
initial scan, scan progress, root mosaic, nested directory, breadcrumb
return, sibling jump, volume switch, cached revisit, explicit rescan,
selection, mark/unmark, review, cancel, confirm and removal result. Confirm
tile navigation, breadcrumb return and cached volume revisit issue no scan
request. Confirm cache miss scans once and failed/cancelled rescan preserves
the prior tree. Confirm Vue receives bounded view payloads rather than a
serialized full tree. Preserve core removal guards and free-space attribution.
Run focused CLI/protocol and desktop coverage, Rust lint/build and desktop
build; manually inspect the running app against the source screen at normal
and large viewport sizes. Validate RFC 0004 and RFC 0006 consistency after
implementation.
