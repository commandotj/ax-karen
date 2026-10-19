# RFC 0032: Check for updates and prompt to install (child of 0031)

**Status:** Approved

**Parent:** [0031](0031-in-app-auto-update.md)

## Summary

On startup the desktop app checks the latest release for a newer version and, if one exists, shows a prompt. Confirming downloads, verifies and installs the update, then restarts the app.

## Problem

Nothing in the app knows about releases. The Tauri updater plugin is not installed, no update endpoint or public key is configured, and the window has nowhere to show an update.

## Goals

- Add the Tauri updater and process plugins and the matching capabilities (`updater:default`, `process:allow-restart`), and nothing else new to the shell.
- Configure the endpoint `https://github.com/commandotj/ax-karen/releases/latest/download/latest.json` and the update public key in `tauri.conf.json`.
- The Vue app checks once at startup, in the background; a failed or offline check is silent apart from a console message.
- A newer version shows a prompt with the version (no notes link: opening external URLs needs extra shell permissions); "Install and restart" downloads, installs and relaunches, "Later" dismisses it until the next launch.
- Copy is localized in all nine locales.

## Non-goals

- Silent install, periodic re-check, or a manual "Check now" menu.
- Producing signed artifacts or `latest.json` (RFC 0033).
- Business logic in the Tauri host: it only registers the plugins (invariant 10).

## Design

- `src-tauri/Cargo.toml` and `src/main.rs`: register `tauri-plugin-updater` and `tauri-plugin-process`. `src-tauri/capabilities/default.json` gains the two permissions.
- `apps/desktop/package.json`: `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process`.
- `tauri.conf.json`: `plugins.updater` with `endpoints` and `pubkey`. The public key is produced by the maintainer with `pnpm tauri signer generate`; the private key only goes into the `TAURI_SIGNING_PRIVATE_KEY` repository secret.
- `App.vue` or a small component under `views/` or `components/`: calls `check()` on mount only inside Tauri, holds the prompt state, and drives download progress, install and `relaunch()`. Locale keys under a new `update` group in all nine locales.
- Version comparison and signature verification are the plugin's job; the app adds no logic for them.

## Acceptance

- Tests with the plugin modules mocked: no prompt when no update; prompt with version when one exists; Later hides it; Install calls download and install then relaunch; a rejected check shows nothing and does not throw; nothing runs outside Tauri.
- Locale test: every locale defines the new keys.
- `pnpm build`, `pnpm lint`, `pnpm test` keep the 100% line coverage gate; `cargo test` and `clippy` pass for `src-tauri`.
- Manual: an installed older build shows the prompt against a release that carries signed artifacts (needs RFC 0033 and a key).
