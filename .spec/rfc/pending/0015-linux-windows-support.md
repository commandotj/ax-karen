# RFC 0015: Linux desktop support

**Status:** Pending

## Summary

Verify and support Karen's standalone CLI and desktop app on Linux.
Windows is tracked separately in [RFC 0016](0016-windows-support.md).
Karen is macOS-first: RFC 0004 accepts macOS only. This RFC holds the Linux
work that RFC 0004 deferred, so it can be scheduled when Linux users matter.

## Problem

CI and release workflows already define Linux build matrices, but
nothing proves they pass, and several behaviors are written only for macOS or
untested elsewhere: Full Disk Access guidance, `reveal`, `choose_directory`,
Trash support, mount attribution and the `~/.karen/config.toml` location.

## Goals

* CI passes on Linux for `cargo xtask lint`, `cargo xtask test`,
  `pnpm test`, `pnpm build` and `pnpm tauri`.
* Release artifacts for each platform contain the standalone `karen` CLI and
  the desktop bundle with a matching-architecture sidecar.
* Platform-specific CLI behavior works or reports `unsupported` explicitly:
  reveal in the file manager, directory chooser, privacy hints, Trash.
* Installer upgrade and uninstall behavior is verified, including that user
  settings in the home directory survive.

## Non-goals

* Changes to macOS behavior or scan, mark and removal semantics.
* New features beyond parity with the macOS build.

## Design

Start from the existing `.github/workflows/ci.yml` and `release.yml` matrices.
Run them, record failures per platform, then fix in the owning crates
(`monitor-disk`, `apps/cli`). Keep the Tauri shell free of platform business
logic.

## Acceptance

Green CI on Linux; downloadable release artifacts launched once
on each platform; a written list of behaviors that report `unsupported`.
