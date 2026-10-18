# RFC 0008: Report per-process memory usage (child of 0001)

**Status:** Approved

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Add read-only per-process memory sampling to Karen's CLI monitor stream so the
Memory tab can show which processes currently hold the most memory. Keep
collection in `monitor-memory`/`monitor-core` and deliver bounded process rows
to the Vue shell, with platform-specific meaning documented.

## Problem

Karen currently reports only whole-machine memory totals. Users cannot tell
which processes account for memory use, leaving the Memory view without the
per-process breakdown required by the monitoring design. RFC 0003 excluded
process monitoring, and RFC 0005 covers CPU process usage only.

## Goals

Collect each visible process's stable identifier, display name and memory
usage. Return a bounded list ordered by descending memory use on the normal
resource sampling interval. Define consistent unavailable, permission-limited
and process-exit behavior across Linux, macOS and Windows. Keep total physical
memory readings intact.

## Non-goals

Process CPU sampling (RFC 0005), process termination/control, per-process
virtual-memory breakdown, swap attribution, alerts, history/recording, remote
telemetry, privileged collection, and UI layout (RFC 0007).

## Design

Extend `monitor-memory`'s sampler to refresh process memory readings using the
existing `sysinfo` dependency and return a bounded, normalized process list
alongside current physical-memory totals. Extend `monitor-core`'s resource
snapshot and the CLI `resources` event with process rows. Document the exact
memory metric exposed by `sysinfo` on each supported platform; do not combine
incompatible metrics under one label. Sort descending by bytes, then PID for
stable ties. Bound the serialized list to the top 20 processes. Exclude
non-finite or unavailable values, and use a safe fallback when process names
cannot be read. Keep the Tauri shell as pass-through and leave presentation to
RFC 0007.

`Process::memory()` reports resident set size on Linux and macOS (Linux
resident pages from `/proc`, macOS `pti_resident_size`) and working set size
on Windows (`WorkingSetSize`). The UI labels this value `RSS / WORKING SET`;
it is physical resident memory, not virtual size or a sum that can be compared
directly with system-wide used memory. `sysinfo` exposes this as an unsigned
byte count and uses zero when a platform query fails; omit zero-valued rows so
an inaccessible process is not shown as a measured zero. The list remains
best-effort and unprivileged: inaccessible processes may be absent.

## Acceptance

On Linux, macOS and Windows, the CLI resource event returns at most 20
processes, each with stable PID, display name and non-negative memory bytes,
ordered by memory descending with deterministic ties. Existing total memory
fields and resource cadence remain compatible. Permission-denied, warm-up,
process-exit and unavailable readings do not crash sampling or mislabel
unknown data. Focused sampler/protocol coverage, Rust lint/build and manual
stream inspection pass. The Memory tab integration is verified under RFC
0007 after both RFCs are approved.
