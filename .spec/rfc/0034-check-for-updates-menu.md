# RFC 0034: Add a Check for Updates menu item (child of 0001)

**Status:** Approved

**Parent:** [0001](completed/0001-karen-system-resource-app.md)

## Summary

Add a "Check for Updates…" item to the tray menu. It brings the window forward, checks for a newer release, and always answers: the install prompt when one exists, otherwise a short "up to date" or "could not check" notice.

## Problem

RFC 0032 checks for updates once at startup and stays silent when there is nothing to show, so a user cannot ask for a check and gets no feedback. The tray menu is the only application menu Karen defines.

## Goals

- A tray menu item, localized like the other tray labels, that shows the window and starts a check.
- A manual check that finds an update shows the existing prompt; one that finds none says Karen is up to date and names the running version; one that fails says the check could not be completed.
- The startup check stays silent.
- The window shows the running version beside the brand (`v0.13.0`), so the user can compare it with what a check reports.

## Non-goals

- A macOS application-menu item or a menu-bar item outside the tray; the app defines no such menu today.
- Periodic background checks, silent install, or any change to the prompt's install behavior.
- Starting a second check while one is running or an install is in progress.

## Design

- `src-tauri/src/main.rs`: a `check-update` tray item stored in `TrayState`, its label set through a new `check_update_label` argument of `update_tray_status` like the other items. Its handler shows and focuses the main window and emits `tray-check-update`. The host does no checking.
- `App.vue`: `syncTrayStatus` passes `checkUpdateLabel: t("update.checkMenu")`.
- `UpdatePrompt.vue`: listens for `tray-check-update`, ignores it while a check or install is running, runs the same `check()` as startup, and shows an informational notice (with a dismiss button) for "up to date" or "failed". The running version comes from `getVersion()`.
- `App.vue` reads the running version with `getVersion()` inside Tauri and shows it in the masthead; nothing is shown outside Tauri or before it resolves.
- `update_tray_status` already had 12 arguments, over clippy's limit of 7, and this change would add a 13th. Its labels move into one `TrayLabels` struct (serde, camelCase) sent by the frontend as `{ labels }`, and its tooltip fallback becomes a small tested function.
- Locale keys `update.checkMenu`, `upToDate`, `checkFailed`, `dismiss` in all nine locales and the schema type.

## Acceptance

- Component tests with the plugins mocked: the event triggers a check; update found shows the prompt; none found shows the up-to-date notice with the version; a rejected check shows the failure notice; the notice dismisses; the event is ignored during a check and during an install; the listener is removed on unmount; the startup check stays silent.
- `App.test.ts`: the tray sync sends the localized `checkUpdateLabel`; the masthead shows `v` plus the running version inside Tauri and nothing outside it.
- Locale test: every locale defines the new keys.
- Rust unit tests for the `TrayLabels` camelCase contract and the tooltip fallback; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass for `src-tauri`.
- `pnpm test`, `pnpm lint`, `pnpm build` and the 100% line-coverage gate pass; `cargo test` and `clippy` pass for `src-tauri`.
- Manual: choosing the tray item in the running app shows the expected notice or prompt. The tray handler itself has no automated test.
