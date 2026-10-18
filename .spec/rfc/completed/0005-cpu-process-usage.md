# RFC 0005: CPU process usage monitor (child of 0001)

**Status:** Implemented

**Parent:** [0001](../0001-karen-system-resource-app.md)

## Summary

Expose a bounded, live list of processes ranked by CPU usage on Karen's
Runtime tab. The CLI owns process sampling and sends process rows through its
existing `--stream-json` protocol; the Tauri shell remains a transport and the
Vue UI renders the result.

## Problem

The Runtime tab currently reports only whole-machine CPU utilization. That
shows load but cannot identify which process causes it. RFC 0003 explicitly
excluded per-process monitoring, so this capability needs its own RFC and
acceptance boundary.

## Goals

Show the highest CPU-consuming processes, with process name, PID, and CPU
percentage. Refresh on the existing bounded resource-sampling cadence, keep
the list capped, and handle process names or CPU readings unavailable from the
operating system.

## Non-goals

Process memory, process termination or control, alerts, history/recording,
remote telemetry, elevated collection, and changes to disk scanning or
deletion behavior.

## Design

Extend `monitor-cpu` to sample per-process CPU using the existing `sysinfo`
dependency and emit normalized rows through `monitor-core`. Add a versioned
resource event to the `karen --stream-json` protocol and update the Vue Runtime
tab to render the bounded, descending-CPU list. Keep process collection in the
CLI/core path; Tauri only forwards stream events. Preserve the current total
CPU and memory fields and the stream protocol's backward-compatible handling
of unknown events. Label process CPU with `sysinfo`'s per-core percentage
semantics: one fully used core is 100%, so a process may exceed 100% on a
multi-core system. Treat first-sample CPU deltas as unavailable until the
collector has a baseline; exclude non-finite or negative readings, use a safe
fallback for empty process names, and sort ties by PID for stable display.

## Acceptance

On Linux, macOS, and Windows, the CLI stream reports a bounded process CPU
list while retaining existing total CPU and memory readings. Rows contain a
stable PID, display name (with a safe fallback when the OS provides none), and
finite CPU percentage in the documented range. The list is ordered by CPU
usage, updates on the resource cadence, and remains empty or explicitly
unavailable during collector warm-up. The Runtime tab visibly renders the
process list and handles no-process/unavailable states. Existing disk tree
navigation and stream consumers continue to work unchanged. Validate with
focused collector/protocol coverage, lint, and desktop build; manually inspect
the running Runtime tab.
