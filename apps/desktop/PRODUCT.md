# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

People monitoring resource use on their local computer and locating directories
that occupy disk space.

## Product Purpose

Karen monitors processor load, physical memory, mounted volume capacity, and
directory space. Success means users can read current resource pressure and
find where disk space is used without leaving the desktop app.

## Positioning

Karen is a local system monitor whose Tauri window delegates sampling and disk
scanning to the `karen --stream-json` CLI. The CLI owns measurements and disk
tree layout; the Vue interface presents its events.

## Operating Context

Karen runs as a desktop app. Users move between focused resource tabs and
the disk tree, which supports volume selection and directory zoom.

## Capabilities and Constraints

- Resource views cover processor load, physical memory, and mounted volumes.
- Space maps directory sizes and zooms into directories.
- Tauri is a shell around the CLI stream protocol; it does not sample or scan.
- Measurements and scans stay local.
- Deletion and cleanup flows are outside the current monitor UI scope.

## Brand Commitments

- Product name: Karen.
- Visual direction: luxurious steampunk, explicitly requested by the user.
- The interface is a system monitor, not a web page or marketing dashboard.

## Evidence on Hand

- Resource and volume events are implemented in `apps/cli/src/stream_json.rs`.
- Disk scanning and squarified treemap layout live in `crates/monitor-disk`.
- No third-party performance claims, benchmark data, or user testimonials are
  available; do not fabricate them.

## Product Principles

- Keep monitoring local.
- Give each resource a focused view.
- Keep the disk tree directly accessible.
- Keep measurement and scan logic in the CLI and core crates.
