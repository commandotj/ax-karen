# RFC 0016: Windows desktop support

**Status:** Pending

## Summary

Make Karen's standalone CLI and desktop app work and be verified on Windows.
Karen is macOS-first; this RFC tracks the Windows-specific work that has not
been compiled or run: reveal in Explorer, directory chooser, Recycle Bin
removal, custom window chrome, the `~/.karen` settings location, and
x86_64 and arm64 release builds.

## Problem

Windows code exists but has never been proven. `reveal` calls
`explorer /select,<path>` and compiles only under `#[cfg(windows)]`, so no
build or test has ever exercised it. `choose_directory` and
`open_full_disk_access` return `unsupported`. The Windows window config sets
`decorations: false`, while the Vue masthead assumes macOS traffic lights or
its own window controls. Release matrices for `windows-2025` and
`windows-11-arm` are defined but have no recorded green run.

## Goals

* `cargo xtask lint`, `cargo xtask test`, `pnpm test`, `pnpm build` and
  `pnpm tauri` pass on `windows-2025` and `windows-11-arm`.
* `reveal` opens Explorer with the path selected; quoting is correct for paths
  with spaces and special characters; failures report `reveal_failed`.
* `choose_directory` works through a native folder dialog or reports
  `unsupported` with a documented fallback.
* Removal to the Recycle Bin and permanent deletion keep every guard in
  `monitor-disk`, including drive-root, user-profile and symlink checks.
* Custom window controls (minimize, maximize, close) and dragging work with
  `decorations: false`.
* `%USERPROFILE%\.karen\config.toml` is read and written by the CLI and
  survives app upgrade and uninstall.
* Release artifacts contain `karen.exe` and a bundle with a matching-
  architecture sidecar; installer upgrade and uninstall are verified.

## Non-goals

* macOS or Linux behavior, scan accounting, mark and removal semantics.
* Code signing and store distribution beyond what the release workflow needs.

## Design

Run the existing CI and release matrices on Windows first and record each
failure. Fix scanner and removal behavior in `crates/monitor-disk`
(`windows.rs`, `mft.rs`, `removal.rs`), protocol behavior in `apps/cli`, and
window controls in `apps/desktop/src`. Keep the Tauri shell free of business
logic. Add a Windows-only test for path quoting in `reveal` that does not
launch Explorer.

## Acceptance

Green Windows CI on both architectures; a release artifact installed, launched,
upgraded and uninstalled once on x86_64 and on arm64 hardware or a runner;
a written list of behaviors that report `unsupported`.
