# RFC 0007: Make Karen a usable live system monitor (child of 0001)

**Status:** Approved

**Parent:** [0001](completed/0001-karen-system-resource-app.md)

## Summary

Redesign Karen's monitoring views as a usable live system monitor with a
highly visual Runtime instrument panel and dedicated CPU, Memory and Disk Tree
detail tabs. Runtime visualizes every supported resource together; a compact
readout table may support exact values but must not dominate the screen. CPU
detail shows per-process CPU usage; Memory detail shows per-process memory
usage; Disk Tree shows the original Karen space view specified by RFC 0006.
Runtime disk instruments open the corresponding Disk Tree/space view. Do not
add a separate Volumes tab. The interface must feel richly luxurious and
ornamented, not clean or minimal.

## Problem

The current desktop is too clean and presents resource readings as sparse
dashboard cards. It lacks the visual richness and instrument-like hierarchy
needed for a luxurious steampunk monitor, while also failing to make
per-process CPU/memory usage and disk space legible. Disk information needs a
direct route into the dedicated space treemap.

## Goals

Make all resources Karen currently supports legible, visual and actionable at
a glance. Runtime shows CPU utilization, physical-memory usage, and every
mounted disk/volume's used and available space together. CPU and memory
instruments include concise process-usage summaries; their detail tabs show
the full bounded process lists. Pair gauges, live traces, segmented capacity
rings/bars, engraved scales and luminous state indicators with exact numeric
readouts. Disk instruments are clickable and open the matching Disk Tree/space
view. Disk Tree remains a dedicated view. The app opens on Runtime and does
not start a filesystem scan until the user opens Disk Tree.

## Non-goals

Process CPU collection and stream schema (RFC 0005), Disk Tree behavior and
cached navigation (RFC 0006), process memory collection/schema (RFC 0008),
disk I/O throughput, network telemetry, alerts, historical recording, remote
monitoring, new sampling backends, and changes to deletion or filesystem
policies.

## Design

Restructure `apps/desktop/src/App.vue` into Runtime, CPU, Memory and Disk Tree
views and update `apps/desktop/src/style.css` to create a dense, richly
ornamented instrument console. Runtime is the default visual monitor panel
and contains every supported resource: CPU, physical memory and each mounted
disk/volume. Show CPU load as a mechanical dial/gauge with a live trace;
memory as a capacity instrument with used/available portions; each mounted
disk as an engraved capacity meter or ring with free-space reading and a click
target into Disk Tree. Include concise CPU-process and memory-process usage
summaries, with full bounded lists in their respective detail tabs. Add a
compact exact-value ledger as supporting detail, never as the dominant visual.
Use layered brass/copper frames, engraved ticks,
filigree corners, inset enamel panels, warm specular highlights, and selective
indicator glow. Preserve readable typography and clear visual hierarchy; do
not simplify the visual treatment into generic flat cards, large whitespace,
or a plain table. CPU detail includes a bounded per-process CPU table supplied
by RFC 0005. Memory detail includes system totals/availability and a bounded
per-process memory table supplied by RFC 0008. Disk Tree owns its dedicated
layout and controls from RFC 0006. Do not create a separate Volumes tab.

Use CLI stream data only; Vue formats and displays it. Keep existing
three-second sample cadence unless RFC 0003 or RFC 0005 implementation
constraints require a documented adjustment. Preserve the custom title bar,
macOS traffic lights, tab navigation, accessibility labels and luxuriously
ornamented steampunk palette. Do not automatically launch a disk scan from
Runtime or Volumes.

## Acceptance

On launch, Runtime is selected and visualizes every supported resource:
system CPU, physical memory, and every mounted disk/volume. Each resource has
distinct instrument graphics, exact readings, and concise process-usage
summaries where process data applies. CPU and Memory detail tabs show bounded
per-process usage lists with stable identity and explicit unavailable states.
Disk instruments open their matching Disk Tree/space view. Runtime visibly
reads as a richly ornamented luxury steampunk
instrument panel, not a clean dashboard or table. Disk Tree retains original
Karen behavior; no Volumes tab exists. Launching or switching monitor tabs
does not start a disk scan. The window keeps custom chrome and macOS traffic
lights. Verify live updates, empty/unavailable states, narrow and large
window layouts, and keyboard-accessible tabs. Build the desktop and manually
inspect each view; run `specify validate` after dependent RFCs 0005, 0006 and
0008 are approved and integrated.
