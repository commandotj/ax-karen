# RFC 0017: Reach full test coverage with a CI gate

**Status:** Approved

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
