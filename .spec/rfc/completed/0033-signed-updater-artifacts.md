# RFC 0033: Publish signed updater artifacts and latest.json (child of 0031)

**Status:** Implemented

**Parent:** [0031](0031-in-app-auto-update.md)

## Summary

Make the Release workflow build signed updater artifacts for macOS arm64 and Linux x86_64, generate `latest.json`, and upload it with the other assets so the app can find updates at the `releases/latest/download` URL.

## Problem

The workflow publishes only DMG, deb and AppImage plus checksums. The Tauri updater needs, per platform, an updater archive with a `.sig` signature and a `latest.json` manifest that points at them.

## Goals

- Release builds set `bundle.createUpdaterArtifacts` and sign with the `TAURI_SIGNING_PRIVATE_KEY` secret (and its optional password); local `pnpm tauri` builds do not need the key.
- macOS uploads `Karen.app.tar.gz` and its `.sig`; Linux uploads the AppImage and its `.sig`.
- The publish job builds `latest.json` with `version`, `notes`, `pub_date`, and `darwin-aarch64` and `linux-x86_64` entries (`signature` from the `.sig`, `url` of the uploaded asset), validates it, and uploads it to the same release.
- `SHA256SUMS` covers the new files.
- The workflow fails early with a clear message when the signing secret is missing.

## Non-goals

- The in-app check and prompt (RFC 0032).
- Windows, other architectures, or a second update channel.
- Generating or storing the signing key; the maintainer does that.

## Design

- `release.yml`: pass `--config '{"bundle":{"createUpdaterArtifacts":true}}'` and the signing env to the release `tauri build`; extend the macOS and Linux packaging steps to copy the updater artifacts into `dist/`.
- A small `scripts/updater-manifest.mjs` (Node, no dependencies, covered by a `node --test` file) assembles and validates `latest.json` from the staged files and tag; the publish job calls it before upload.
- A preflight step checks the signing secret like the Apple preflight does.

## Acceptance

- Unit tests for the manifest script: correct URLs and platforms, missing signature fails, version and tag mismatch fails.
- The workflow parses and the new steps are gated on the release job.
- Verified locally on macOS arm64: `pnpm tauri --config '{"bundle":{"createUpdaterArtifacts":true}}'` with a signing key produces the DMG, `Karen.app.tar.gz` and `.sig`, and `scripts/updater-manifest.mjs` builds a correct `latest.json` from them. The `TAURI_CONFIG` environment variable is not honoured by tauri-cli 2.12.3, so `--config` is used.
- A real release run produces `latest.json` and the `.sig` files; that is only observable on a release, and the RFC stays Approved until it has been seen.

## Closing note

Closed after PR #21 merged, on automated tests and a local macOS arm64 build and manifest check. Not yet observed on a real release: the Linux AppImage `.sig` name, a signed release run, and an installed older build finding and installing an update. Reopen via a new child RFC if the first release shows a problem.
