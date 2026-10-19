# RFC 0001: Karen system resource app (Umbrella)

**Status:** Implemented

**Type:** Umbrella

## Summary

Build Karen as a Rust desktop resource-monitoring app that combines disk-space analysis and guarded cleanup with CPU and memory monitoring. This umbrella tracks the desktop migration and resource-monitoring work through separate child RFCs.

## Problem

The product direction spans independently reviewable concerns: migrating the desktop app to Karen's CLI and Tauri/Vue architecture, and adding live CPU/memory views alongside disk-space management.

## Goals

Both child RFCs are implemented and their acceptance criteria pass. Existing disk scanning, mark/review/removal safeguards, and supported-platform behavior remain intact.

## Non-goals

Kernel tuning, process termination, automatic cleanup, privileged collection, remote telemetry, and unrelated platform expansion. New concerns require a new RFC.

## Children

| RFC | Concern |
|-----|---------|
| [0034](../0034-check-for-updates-menu.md) | Add a Check for Updates menu item |
| [0030](0030-tag-release-pr-after-publish.md) | Mark the merged release PR as tagged after publishing |
| [0029](0029-larger-default-window.md) | Enlarge the default window size |
| [0028](0028-titlebar-drag-maximize.md) | Fix custom title bar drag, cursor and double-click maximize |
| [0027](0027-disktree-click-context-menu.md) | Clarify Disk Tree click and context-menu actions |
| [0014](0014-replace-xtask-with-mjs.md) | Replace Rust xtask runner with root mjs script |
| [0013](0013-redesign-icon-logo-designer.md) | Redesign application icon with Logo Designer skill |
| [0012](0012-top-bar-tray-icon.md) | Add top-bar system tray icon and menu |
| [0011](0011-network-tab-monitoring.md) | Add network telemetry and Network tab |
| [0010](0010-battery-tab-monitoring.md) | Add battery telemetry and Battery tab |
| [0009](0009-disktree-ui-parity.md) | Restore Disktree UI feature parity in Karen |
| [0008](0008-per-process-memory-usage.md) | Report per-process memory usage |
| [0007](0007-usable-system-monitor-experience.md) | Make Karen a usable live system monitor |
| [0006](0006-karen-disk-tree-ui-migration.md) | Migrate the complete Disk Tree interface and interactions to Vue 3 |
| [0005](0005-cpu-process-usage.md) | CPU process usage monitor |
| [0004](0004-tauri-vue3-desktop.md) | Migrate Karen desktop UI from GPUI to Tauri and Vue 3 |
| [0003](0003-cpu-memory-resource-monitor.md) | CPU and memory monitoring with disk space management |
| [0002](../rejected/0002-rebrand-karen.md) | Superseded application identity transition |

## Acceptance

Close when both listed children are implemented and their acceptance evidence is recorded. Keep each child independently scoped; never reopen this umbrella for follow-up features.
