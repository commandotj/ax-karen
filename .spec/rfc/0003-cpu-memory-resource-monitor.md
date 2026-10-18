# RFC 0003: CPU and memory monitoring with disk space management (child of 0001)

**Status:** Approved

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Add live CPU and memory monitoring to Karen's existing disk-space management view. Show CPU and memory meters beside the disk panel while retaining the existing disk treemap, marked-path review, deletion guards, and free-space projections.

## Problem

Karen's intended scope is system resource management, but the current app only explores and frees disk space. CPU and memory readings complete the requested resource overview without weakening the disk workflow's safety invariants.

## Goals

Provide readable current CPU utilization and memory usage, refreshed on a bounded interval, alongside navigation to existing disk-space management. Collection is read-only and unprivileged. Define per-platform collection semantics for Linux, macOS, and Windows, including unavailable readings and platform-specific memory definitions. Keep sampling and presentation separate from filesystem scanning.

## Non-goals

Per-process monitoring or termination, performance tuning, alerts, history/recording, remote telemetry, privileged access, swap/cache clearing, and changes to disk deletion behavior.

## Design

Add a small platform-neutral resource sample model backed by the existing cross-platform `sysinfo` dependency. Keep sampling in its own module and render compact CPU and memory meters in the side panel beside disk space. Do not add a separate screen or embed resource polling into the disk scan or treemap. Use bounded sampling, handle CPU counter warm-up and unavailable memory explicitly, and retain no history. Preserve all current disk-space projections and removal guards.

## Acceptance

Unit-test CPU delta calculations, memory totals/availability handling, warm-up, and unavailable samples with deterministic fixtures. Add window-harness coverage for navigation, rendering valid and unavailable readings, and returning to disk management. Run existing core/app tests, lint, and builds on Linux, macOS, and Windows. Manually confirm readings update while the disk tree remains usable and no collector requires elevated privileges.
