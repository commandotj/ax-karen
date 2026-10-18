---
name: karen-design
description: Design, implement, or review Karen desktop resource-monitor interfaces. Use when changing Runtime, CPU, Memory, Disk Tree, app chrome, or visual styling in this repository.
---

# Karen interface design

Design Karen as a luxurious steampunk system monitor: a desktop instrument, not
a website, marketing page, or generic dashboard. Preserve the user's request
for ornate visual richness. Do not simplify it into flat cards, whitespace, or
a table-led layout.

## Before changing UI

1. Read `apps/desktop/PRODUCT.md` and the relevant RFCs in `.spec/rfc/`.
2. Treat RFC status as a hard boundary. Draft RFCs are design context, not
   implementation approval. Follow the repository's RFC approval workflow
   before implementing RFC-scoped behavior.
3. Use `assets/screenshot.png` as the visual reference for Disk Tree
   structure and information hierarchy. Match its workflow and composition;
   apply Karen's luxurious brass, copper, enamel, engraving, and restrained
   instrument glow.
4. Preserve `.impeccable/` at repository root. Use its design notes when
   present; do not move or replace that directory.
5. Keep the Tauri window a shell around the `karen --stream-json` CLI. The CLI
   owns measurements, scans, tree state, and filesystem actions. Vue renders
   CLI events and sends user actions through the existing protocol.

## Information architecture

Use a custom title bar and keep macOS traffic lights visible. Do not restore
the operating-system default title bar. Make the app open on **Runtime**.

Use focused tabs:

- **Runtime**: visual overview of every supported resource on one instrument
  console: system CPU, physical memory, and every mounted volume. Show live
  state, exact readings, capacity or utilization, and concise process-use
  summaries where data exists. Use gauges, traces, engraved scales, segmented
  meters, indicators, and compact numeric readouts. Graphics lead; a plain
  ledger or cards must not dominate.
- **CPU**: detailed, bounded per-process CPU usage, with stable process
  identity, clear units, sorting, and explicit unavailable states.
- **Memory**: physical-memory totals and detailed, bounded per-process memory
  usage, with metric labels matching the CLI's platform-specific meaning.
- **Disk Tree**: the complete disk space-finding work surface. Do not add a
  separate Volumes tab. Clicking a Runtime volume instrument opens Disk Tree
  for that volume.

Do not start a filesystem scan on launch or when switching between Runtime,
CPU, and Memory. Scan only after entering Disk Tree or explicitly requesting a
rescan.

## Disk Tree reference contract

Preserve the source screen hierarchy shown in `assets/screenshot.png`:

1. Top controls and absolute breadcrumb trail, including direct breadcrumb
   navigation and sibling jumps.
2. Scan totals, filters, and category legend above the treemap.
3. Large treemap as the main work surface, retaining proportions, labels, and
   selection behavior.
4. Side panel for selection details, marks, review entry, and volume space.
5. Keyboard help and scan status along the bottom.

Preserve the full workflow: volume selection, cached tree navigation, click to
descend, breadcrumb return, sibling navigation, selection, mark/unmark,
review, guarded removal confirmation, and explicit rescan. Navigation within
a completed scan uses the cached tree. Never make a volume click discard the
visible tree or silently trigger a fresh scan. Keep paths absolute when
resolving tiles, breadcrumbs, and marks. Keep deletion safeguards owned by the
CLI/core.

## Visual language

- Build an instrument console with layered dark enamel, substantial brass and
  copper framing, engraved dividers and scales, rivets or fine corner
  ornament, selective highlights, and restrained status illumination.
- Give each resource a distinct, legible instrument. Ornament must reinforce
  reading and interaction, never obscure values or consume the treemap.
- Maintain dense information hierarchy suitable for continuous monitoring.
  Use stable alignment, readable labels, and explicit units; do not make users
  infer whether values are percentages, bytes, rates, or totals.
- Use custom-designed controls and dropdowns that match the console. Avoid
  browser-default select menus, unstyled native popovers, and platform-default
  chrome. Preserve the traffic lights.
- Keep pointer interaction as the default desktop cursor. Use text cursors only
  inside actual text-entry controls.
- Make tabs, instruments, treemap tiles, breadcrumbs, and review actions
  keyboard accessible with visible focus and meaningful accessible names.
- Handle empty, loading, stale, permission-limited, and unavailable telemetry
  as explicit states. Never fabricate readings or imply a scan completed when
  it did not.

## Review checklist

Before presenting a UI change, check:

- Does Runtime visualize CPU, physical memory, and all mounted volumes
  together, with exact readings and applicable process summaries?
- Do CPU and Memory detail views show per-process usage rather than only
  machine totals?
- Does each Runtime disk instrument jump to the matching Disk Tree volume?
- Does Disk Tree retain the source hierarchy and cached navigation behavior?
- Does launch avoid scanning disks until Disk Tree is opened?
- Are custom chrome, traffic lights, ornate steampunk treatment, readability,
  and keyboard paths intact?
- Are Draft RFC requirements kept separate from approved implementation scope?

Inspect the rendered app at practical window sizes when visual tooling is
available. Report what was inspected and what remains unverified; a build or
static check alone does not establish visual correctness.

## Authority

Repository product definition and RFCs are authoritative for behavior. This
skill defines the standing visual and interaction direction; it does not
override RFC approval, CLI ownership, or disk safety invariants.
