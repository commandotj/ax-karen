# RFC 0017: Reach full test coverage with a CI gate

**Status:** Implemented

## Summary

Raise automated coverage of Rust and the Vue interface from today's measured
levels to 100% of lines in testable code, with every exclusion written down,
and fail CI when coverage drops. Measured on macOS on 2026-10-17: Rust 87.07%
lines (87.17% regions, 870 functions with 113 never run), Vue 47.79% lines.

## Problem

There is no coverage measurement in the repository and no gate, so coverage
only goes down. Measured gaps:

| Area | Lines covered |
| --- | --- |
| `apps/cli/src/main.rs` | 17% |
| `monitor-disk/src/access.rs` | 48% |
| `monitor-core/src/lib.rs` | 61% |
| `apps/cli/src/stream_json.rs` | 71% |
| `monitor-disk/src/removal.rs` | 84% (destructive code) |
| `monitor-disk/src/scan.rs` | 87% |
| `App.vue` | 58% |
| `treemap.ts` | 12% |
| `cli.ts`, `main.ts` | 0% |

The least covered code includes the destructive path (`removal.rs`), the
protocol loop and the canvas painter and hit testing.

## Goals

* One documented command per language prints line coverage:
  `cargo llvm-cov --workspace` and `pnpm test -- --coverage`.
* 100% line coverage of Rust and TypeScript, except code listed in an
  exclusion table in this RFC's successor notes with a reason each (for
  example macOS-only FFI that needs a real Trash, `osascript` dialogs,
  `#[cfg(windows)]` code on a macOS runner, `fn main` glue).
* A CI job fails when line coverage falls below the recorded threshold, and
  the threshold only moves up.
* Priority order: `removal.rs` and `disk_marks.rs`, then `stream_json.rs`,
  `monitor-core/src/lib.rs`, `treemap.ts` drawing and hit testing, then the
  remaining `App.vue` branches (history, zoom, filter, help, export, power
  menu, Full Disk Access hint, error and cancel paths).
* Tests assert behavior through protocol events or rendered output, not
  private functions; no test depends on a real file manager, dialog or Trash.

## Non-goals

* Branch or mutation coverage targets, performance tests, Linux and Windows
  coverage (RFCs 0015 and 0016), visual regression screenshots.
* Rewriting production code only to make it testable, beyond small seams such
  as injecting a command runner for `reveal` and `choose_directory`.

## Design

Rust: `cargo-llvm-cov` with `llvm-tools`; on Homebrew Rust set `LLVM_COV` and
`LLVM_PROFDATA` to the Homebrew LLVM binaries. Frontend: `@vitest/coverage-v8`
(already added), measuring `src/**` and excluding test files. Treemap painter
tests use a fake 2D context that records calls so fills, outlines and labels
can be asserted. Add a coverage step to `ci.yml` on macOS and publish the
number in the job summary. Record exclusions in one table and re-check it
each time the threshold is raised.

## Acceptance

Both coverage commands run from the documented root commands; coverage is 100%
of lines except the listed exclusions; CI fails on a deliberate drop; every
exclusion has a reason; `specify validate` and `sync-check` pass.

## Results and exclusions

Measured on macOS, 2026-10-17, after the work: Vue 100% of lines (79 tests),
Rust 96.91% of lines (267 tests, 0 failures). Commands: `pnpm coverage:rust`
(`cargo llvm-cov`, gate `--fail-under-lines 96`) and `pnpm coverage:web`
(vitest, gate `lines: 100`); `pnpm coverage` runs both and a macOS CI job runs
it. The gate was shown to fail locally (`--fail-under-lines 99` exits 1). The
CI job itself has not run on GitHub. Homebrew Rust needs `LLVM_COV` and
`LLVM_PROFDATA` set to Homebrew LLVM; CI uses `llvm-tools-preview`.

Rust lines still uncovered, with reasons. Raise the threshold when any entry is
removed; RFC 0018 covers the OS-dependent entries.

| Where | Reason |
| --- | --- |
| `monitor-disk/removal.rs` 1444-1454, 1418-1420, 1503-1508, 2597-2612 | Real NSFileManager Trash, `trash-put`, `gio`, real home trash; RFC 0018 |
| `removal.rs` 254, 430-440, 462, 608-612, 730-735, 750 | Windows, Linux `/proc` and other-OS branches, not compiled or run on macOS; RFCs 0015, 0016 |
| `removal.rs` 713, 1036-1042; `scan.rs` 307-308, 1029 | Thread spawn failure cannot be triggered |
| `removal.rs` 1383-1384, 1391-1395 | Race or filesystem that does not report types; needs a real second mount (Linux ignored fixture) |
| `scan.rs` 397-399, 422-424, 478-480, 946, 1054 | Linux-only or Windows-only paths |
| `scan.rs` 451, 527, 505-507, 1154, 1217, 1178-1180, 1039-1040 | Cloud-evicted dirs, deletion races, cancel timing, kernel entry errors, impossible empty root |
| `scan.rs` 2085-2117 | `#[ignore]` whole-disk smoke test |
| `space.rs` 65, 388, 573-582, 621, 641, 818 | Filesystem-dependent, Linux `/proc`, other-OS, failed `mount` |
| `treemap.rs` 366, 383, 405; `macos.rs` 330 | Float degeneracy guards; EINTR retry |
| `apps/cli/stream_json.rs` 378, 393, 1100-1102 | Non-macOS `choose_directory`, Full Disk Access and Linux `xdg-open` branches |
| `stream_json.rs` 40, 204, 228, 603, 901-913, 1000, 1018 | Shutdown race and defensive branches unreachable through the protocol; candidates for deletion |
| `monitor-core/disk_marks.rs` 42, 87 | Unreachable through the public API |
| `monitor-battery` (84.6%) | New crate owned by RFC 0010, still in progress |

Vue: `treemap.ts`, `cli.ts` and `main.ts` are at 100% lines. Branch coverage
is about 95.6% and is not gated.
