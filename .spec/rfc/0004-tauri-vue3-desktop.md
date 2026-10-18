# RFC 0004: Migrate Karen desktop UI from GPUI to Tauri and Vue 3 (child of 0001)

**Status:** Approved

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Replace Karen's GPUI interface with `Karen.app`, a Tauri 2 + Vue 3 desktop
shell around the standalone Karen CLI. The CLI is sole owner of application
state and all work. Karen.app starts the bundled CLI in `--stream-json` mode
and forwards its JSON stream between the Vue interface and CLI.

## Problem

Karen currently depends on GPUI for its window and interface. Product shape
is a useful standalone CLI first, then Karen.app as its desktop shell. Tauri
must not reimplement business logic or call backend crates directly. The CLI
needs a machine-readable streaming mode that can serve an interactive app
without copying the complete tree to Vue or keeping two copies of application
state.

## Goals

* Ship `Karen.app` as a Tauri 2 desktop shell with Vue 3 and TypeScript under
  `apps/desktop/`.
* Rename the disk library package to `monitor-disk`; add
  `monitor-core` for app-level orchestration, `monitor-memory` for memory
  sampling, and `monitor-cpu` for CPU sampling. Keep each crate limited to its
  named responsibility.
* Keep standalone `karen` CLI in `apps/cli/` as the only application backend.
  `Karen.app` starts the bundled CLI with `--stream-json`; Tauri only owns the
  window, CLI process lifecycle and byte/JSON-line forwarding to Vue.
* Preserve the current user-visible flows: CPU and memory meters, disk free
  space, treemap navigation and zoom, filters, selection details, marks,
  review, confirmation, safe removal, and scan controls.
* Keep filesystem semantics and destructive-operation guards in
  `monitor-disk`, called only by the CLI; neither Tauri nor Vue performs file
  operations.
* Keep scan and removal work in CLI process. Stream progress and bounded view
  data to Vue; do not serialize or clone the complete tree for each redraw or
  interaction.
* Preserve supported Linux, macOS and Windows builds, existing command-line
  scan arguments under the `karen` command, installation upgrades and user
  settings. Use Karen's new application identity consistently.
* Retain projectify layout boundaries: Rust reusable code in `crates/`,
  executable desktop app in `apps/desktop/`, TypeScript packages in
  `packages/` only when shared modules actually exist, and pnpm scripts at the
  root to coordinate Cargo and frontend commands.

## Non-goals

* New monitoring features, process lists or process termination, resource
  history, remote telemetry, auto-cleanup, and privileged collection.
* Changes to scan accounting, mount attribution, marks semantics, or removal
  policy except fixes required to preserve those contracts across the bridge.
* Mobile targets, a web-hosted edition, a new CLI product, or a new package
  publishing workflow.
* Branding redesign, registry renaming, and domain changes.
* Compatibility aliases or metadata using the retired product identity,
  desktop entry, release artifact, or runtime compatibility layer. This
  migration removes the old identity rather than preserving it.

## Design

Create `apps/cli/` for standalone `karen` and `apps/desktop/` for `Karen.app`.
Keep Tauri Rust host and `tauri.conf.json` under
`apps/desktop/src-tauri/`; keep Vue/Vite app, views, styles, Canvas treemap
and the pass-through bridge under
`apps/desktop/src/`. Add a root `pnpm-workspace.yaml` and root scripts only to
coordinate the Vue desktop package and existing Cargo commands. Keep
reusable Rust crates in the Cargo workspace: move the current
disk library implementation to `crates/monitor-disk/` and rename
its Cargo package/library to `monitor-disk`; move app lifecycle and
cross-domain orchestration into `crates/monitor-core/`; move memory and CPU
sampling into `crates/monitor-memory/` and `crates/monitor-cpu/`. Put CLI
argument parsing, terminal output and the `--stream-json` entry point in
`apps/cli/`. `monitor-core` owns app state and orchestration and is used by CLI
only; Tauri must not depend on it or any other monitor crate.

Implement `karen --stream-json` as CLI's long-lived machine interface. It
reads JSON request lines on stdin, validates each request, and writes typed
JSON event lines on stdout. Define versioned request and event envelopes for
scan/start/cancel, tree navigation, filters, marks, review, removal
confirmation, CPU/memory samples, errors and shutdown. Send diagnostics only
to stderr. Stream only progress and bounded visible-tile/selection data; keep
the full tree and marks in CLI memory. Vue sends actions to the CLI through
the scoped Tauri Shell sidecar child's stdin and renders events from stdout.
The CLI owns all validation and destructive-operation guards.

Karen.app bundles the matching-platform `karen` executable as a Tauri sidecar.
Vue uses the scoped Tauri Shell plugin to start it with exactly
`--stream-json`, keep one process for the app session, write request lines,
and read event lines. Tauri Rust host contains no app-specific bridge or
business logic; Vue handles the view and CLI handles all work. Scope shell
permissions to this sidecar and fixed mode argument. Do not expose general
shell execution to Vue. Keep frontend code in `apps/desktop/`, not in
`packages/` unless a second consumer creates a real shared package. Do not
create placeholder crates or a generic plugin framework. Retire
the legacy GPUI app crate and dependencies only after the replacement passes
the acceptance checks.

`monitor-core`, called by CLI, owns one application state containing active
scan, `monitor_disk::Node` tree, marks, scan options, CPU/memory samples,
removal plan and task handles. `monitor-disk` owns filesystem scanning, tree
accounting, treemap geometry, disk-space attribution and all removal guards.
`monitor-memory` and `monitor-cpu` own bounded, read-only metric sampling.
Scan and removal tasks remain asynchronous and emit progress/status events.
CLI computes layouts with `monitor-disk` and streams only the bounded visible
result, selection details and sibling lists. Do not add serde derives to the
compact core node solely to serialize the full tree. Use stable absolute paths
for marks and revalidate selected removal targets through the core planner
before confirmation and execution.

Rebuild interface layout and input in Vue, drawing CLI-provided treemap
rectangles and labels in one Canvas surface so thousands of tiles do not
become thousands of DOM elements. Use Tauri APIs only for shell behavior such
as window and sidecar lifecycle. Declare minimum Tauri capabilities needed
for the sidecar; do not grant arbitrary shell execution or filesystem access
to Vue. Preserve platform-specific scan/removal implementations and existing application data
locations. Use the Karen application and bundle identity `me.systembug.karen`
consistently across platforms. Rename the executable and release artifacts to
Karen names, remove compatibility aliases and metadata using the retired
identity, and update
the desktop entry, packaging, docs and workflows to match. This intentionally
changes the macOS bundle identifier and therefore requests fresh macOS
permissions; do not claim old Full Disk Access grants carry over.

Update Cargo workspace membership, release workflow, install/bundle commands,
README build/run instructions and CI to build the CLI sidecar and Vue frontend
before Tauri packaging. The
existing GPUI changes in the working tree are user-owned input; migration must
incorporate or explicitly reconcile them, never discard them implicitly.

## Acceptance

* Cargo package boundaries are `monitor-core`, `monitor-disk`,
  `monitor-memory`, `monitor-cpu`, and the Tauri desktop host; no production
  dependency, command, bundle, desktop entry or release artifact retains the
  retired identity.
* `karen` CLI runs without opening a window. `karen --stream-json` accepts
  versioned JSON requests and emits parseable JSON event lines for the full
  interactive workflow; diagnostics stay on stderr.
* Karen.app bundles and starts `karen --stream-json`; Tauri has no monitor
  crate dependency and contains no scan, tree, metrics, marks or removal
  business logic. Vue actions and CLI events traverse the sidecar stream.
* `pnpm` install, frontend lint/typecheck/build and Tauri dev launch work from
  the documented root commands; `cargo xtask lint`, core tests and Tauri app
  tests pass without GPUI in the production dependency graph.
* Linux, macOS and Windows release builds produce standalone `karen` CLI and
  launchable `Karen.app` artifacts with matching sidecar architecture. `karen`
  scan arguments and installer upgrade/uninstall behavior remain verified.
* Automated frontend/window coverage exercises scan progress, empty/error
  states, navigation/filtering, marking and review, cancellation, and removal
  confirmation. Core deletion-guard tests remain authoritative for what may
  be removed.
* Exercise a large synthetic tree and demonstrate that the frontend bridge
  returns bounded view data without serializing the full `Node` tree on every
  interaction.
* Verify settings remain available after the new identity is installed, and
  check that Tauri capabilities expose no direct arbitrary path deletion.
* Reconcile README, desktop metadata, release names and actual artifact names;
  run `specify validate` and `sync-check` after this RFC is approved and
  implementation is complete.
