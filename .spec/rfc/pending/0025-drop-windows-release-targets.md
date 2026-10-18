# RFC 0025: Disable Windows release targets

**Status:** Pending

## Summary

Disable Windows in the release pipeline, the same way it was disabled in PR
CI: the Windows matrix entries in `release.yml` are commented out, not
deleted, so no Windows binaries ship while the targets stay one uncomment
away from re-enable. Karen's supported platforms are macOS and Linux.

## Problem

`release.yml` builds Windows x86_64 artifacts in both the CLI and desktop
matrices. Windows was disabled in PR CI (commit a6a7671), so nothing tests
the code these artifacts ship, and there is no Windows distribution path —
the Homebrew tap is macOS-only, and the desktop updater contract in
[RFC 0023](../completed/0023-publishify-cicd-pipeline-setup.md) targets macOS
direct distribution. Each release spends a runner and minutes producing
unverified `karen.exe` and an unsigned MSI/zip that nobody should install.

## Goals

* Comment out the Windows entries in the `release.yml` build matrices,
  leaving macOS (arm64, Intel CLI) and Linux (x86_64, arm64 CLI) active.
* No Windows artifacts in future GitHub Releases or `SHA256SUMS`.
* Keep the disabled entries in the file, marked with this RFC, so
  re-enabling is a deliberate uncomment — not archaeology.

## Non-goals

* Deleting the Windows entries or any Windows code; `#[cfg(windows)]` paths
  and the pending Windows-support RFCs (0015, 0016) stay as-is.
* Changing PR CI (Windows already disabled there).
* Declaring Windows unsupported forever; re-enable means restoring CI
  coverage first, then uncommenting the matrix entries.

## Design

* `release.yml`: comment out the `{ name: Windows x86_64, runner: windows-2025, ... }`
  entries in `build-cli` and `build-desktop`, each with a short note
  referencing this RFC. If
  [RFC 0024](0024-cask-only-homebrew-distribution.md) lands first,
  the CLI matrix no longer exists and this reduces to the desktop entry.
* No other changes: `publish` iterates whatever artifacts exist, and the
  Homebrew job is unaffected.

## Acceptance

* A new `v*` tag builds macOS and Linux desktop (and CLI, while RFC 0024 is
  pending) artifacts only; the release contains no `*-pc-windows-msvc` or
  `.msi`/Windows zip assets.
* The Windows matrix entries are visibly commented in `release.yml` with a
  reference to this RFC.
