# Karen — agent guide

Karen is a Rust CLI system monitor with a Tauri 2 + Vue 3 desktop shell. The
CLI owns scanning, telemetry, tree state, marks, review and removal. The desktop
forwards JSON lines and renders bounded data. Do not add a second backend in
Vue or Tauri.

## Product layout

| path | responsibility |
| --- | --- |
| `apps/cli/` | standalone CLI and `--stream-json` protocol |
| `apps/desktop/src-tauri/` | Tauri window and scoped CLI sidecar |
| `apps/desktop/src/` | Vue views, custom window chrome and Canvas treemap |
| `crates/monitor-core/` | CLI-owned application state and orchestration |
| `crates/monitor-cpu/` | read-only CPU sampling |
| `crates/monitor-memory/` | read-only memory sampling |
| `crates/monitor-battery/` | read-only battery and power-supply sampling |
| `crates/monitor-network/` | read-only network interface sampling |
| `crates/monitor-disk/` | scan, tree, layout, space and guarded removal |
| `.agents/skills/karen-design/` | product design constraints and visual review |
| `.impeccable/` | project design reference; keep at repository root |
| `scripts/` | repo-root task scripts: Cargo lint/test, sidecar build, scan benchmarks, mount tests |
| `migration/gpui-reference/` | archived pre-migration GPUI sources; reference only, not compiled |

Monorepo conventions: task orchestration lives at the repository root —
root `package.json` and `scripts/`. Workspace packages declare
only minimal lifecycle scripts (`dev`, `build`); do not add per-package
`scripts/` folders or separate task runners. All Rust and frontend build
output is unified under the root `target/`; sidecars stage at
`target/tauri-sidecars/` and generated Tauri schemas stay untracked.
Nothing generated is committed.

## Commands

```sh
pnpm install
pnpm dev                         # Tauri desktop development app
pnpm build                       # Vue typecheck and production build
pnpm tauri                       # build CLI sidecar and Karen.app bundles
pnpm lint                        # rustfmt --check and clippy -D warnings
pnpm test                        # cargo test --workspace
```

Use `specify` skill's `scripts/specify.mjs` for RFC lifecycle operations.
Resolve script path from the installed skill, then validate RFCs and run `sync-check` after RFC
status or archive changes. Approved RFCs gate implementation; do not hand-edit
`.spec/ROADMAP.md` or `.spec/TASK_TRACKING.md`.

## Invariants

1. **Disk sizes use allocated bytes** (`st_blocks * 512`) unless apparent size
   was requested. Keep platform-specific allocation semantics in the scanner.
2. **Directory totals are derived.** `tree::aggregate` calculates bytes, files
   and directories from leaves. Never patch aggregate values directly.
3. **A directory is built after its scan and every child task finish.** The
   pending sentinel prevents silently dropping unfinished subtrees.
4. **Removal is limited to marked paths under the scanned root.** Preserve
   mount, root, home, parent-of-home and symlink-target guards in
   `monitor-disk/src/removal.rs`.
5. **Marks use absolute paths.** Refresh sizes after scans and discard missing
   paths; never key marks by tile position.
6. **Treemap is one Canvas.** Draw fills first, then selected/hover/mark
   outlines, then labels clipped to each tile's owned header or rectangle.
   Hover changes stroke only; it never changes tile order, DOM order or z-index.
7. **Tile crumbs are absolute from scan root.** Navigation, selection and
   marks must resolve from the original root at every depth.
8. **Zoom changes view transform only.** Keep layout in base coordinates and
   cache geometry until data, viewport or layout options change.
9. **Free-space projection follows mount source.** A mark on a nested mounted
   volume must not inflate free space for the volume shown by the meter.
10. **Tauri is a shell.** No monitor crate dependencies, filesystem access,
    arbitrary shell access or duplicated business logic in the desktop host.

## Change ownership

| change | location |
| --- | --- |
| scan, filters, parallelism | `crates/monitor-disk/src/scan.rs` |
| node structure and totals | `crates/monitor-disk/src/tree.rs` |
| tile geometry | `crates/monitor-disk/src/treemap.rs` |
| removal safety | `crates/monitor-disk/src/removal.rs` |
| free-space measurements | `crates/monitor-disk/src/space.rs` |
| application state and CLI protocol | `crates/monitor-core/`, `apps/cli/` |
| CPU, memory, battery and network sampling | `crates/monitor-cpu/`, `crates/monitor-memory/`, `crates/monitor-battery/`, `crates/monitor-network/` |
| screen layout and controls | `apps/desktop/src/App.vue` |
| treemap paint and hit testing | `apps/desktop/src/treemap.ts` |
| visual tokens and window chrome | `apps/desktop/src/style.css` |

Keep comments focused on reasons. Preserve disk semantics and removal guards
when changing presentation. Follow `.agents/skills/karen-design/SKILL.md` for
visual work and inspect each affected view at practical window sizes.

<!-- birdify:mode:start -->
Birdify mode: auto
Birdify foundation: on
For every authorized coding task, even when the Birdify skill is not activated: read related implementations, callers and applicable design decisions before editing. Resolve questions from source; ask only about material unresolved choices. Keep changes focused and avoid abstractions without concrete benefit. For bug fixes, establish reproduction or concrete failure evidence; favor tests of observable behavior. Verify the original request and report actual checks and limitations honestly. When coordinating with other agents, inspect existing shared task records, record time, base commit and scope there, and clear only your own temporary claims on completion.
These foundation rules do not require reading the skill, creating a map or activity records, starting an architecture review, or requesting approval for routine authorized work. Follow the host's instruction precedence and explicit user directions. The mode below controls the architecture-map workflow only.
Use the Birdify skill before every code-changing task, including small edits, and for planning that explicitly analyzes affected modules. Enter the workflow once per task; update activity before each edit group, not each line.
When active, first inspect existing project maps and report the reusable path or checked locations and why a new map is needed. Follow the skill to validate/reuse the map, preview it, and declare affected modules before editing.
After displaying the map and concrete change plan, wait for user confirmation before implementation. Preparing map and plan artifacts is allowed beforehand. Reuse confirmation of the same displayed plan; confirm material scope changes. Auto mode is not approval. Honor an explicit task-specific waiver.
Planning alone does not authorize code edits or fabricated activity. A one-task request overrides this mode for that task without changing this block. If the skill is unavailable, report it rather than claim its workflow ran.
This is agent guidance, not a filesystem write interceptor. Preserve all instructions outside this managed block.
<!-- birdify:mode:end -->
