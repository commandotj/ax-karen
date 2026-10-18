# RFC 0018: Cover platform-dependent code with integration tests

**Status:** Pending

## Summary

RFC 0017 reaches 100% line coverage of testable code and lists the rest as
exclusions. This RFC tracks the future work of covering those exclusions with
integration tests that run against the real operating system, so the
exclusion table can shrink to nothing.

## Problem

Code that talks to the operating system cannot be covered by unit tests on one
machine without side effects, so it ships unverified:

* Moving items to the system Trash (`NSFileManager` on macOS, Recycle Bin on
  Windows, `trash-put` and `gio trash` on Linux).
* Native dialogs: `osascript` folder chooser, `open -R` reveal, the Full Disk
  Access settings link.
* `#[cfg(windows)]` code such as `explorer /select`, which a macOS runner never
  compiles.
* Program entry glue in `apps/cli/src/main.rs` and Tauri startup.

## Goals

* Trash tests that create files in a scratch directory, remove them with
  `RemovalMode::Trash`, and prove they left the original path and landed in the
  platform Trash, on a runner where that is safe.
* Dialog and reveal code runs behind an injected command runner in unit tests,
  plus one opt-in smoke test per platform that runs the real command on a CI
  runner with a display.
* Windows-only code compiles and runs its tests on a Windows runner (RFC 0016).
* `main.rs` and app startup are exercised by spawning the built binary and
  asserting exit status, `--help` output and a `--stream-json` handshake.
* Every entry in RFC 0017's exclusion table either gains a test or keeps a
  written reason it cannot be tested.

## Non-goals

* Changing platform behavior, adding features, or running destructive tests on
  a developer's real home directory; tests use scratch directories only.

## Design

Introduce a small command-runner seam where production code spawns an OS
command, so unit tests can assert the exact command line. Add a separate
`platform` test target gated by an environment variable so normal `cargo xtask
test` stays side-effect free, and enable it in CI on runners with a login
session. Depends on RFC 0017 for the exclusion table and on RFC 0015 and RFC
0016 for non-macOS runners.

## Acceptance

Each former exclusion has an integration test or a documented reason; the
platform test target passes on macOS in CI and, once RFC 0016 lands, on
Windows; destructive tests touch only scratch directories; `specify validate`
and `sync-check` pass.
