# RFC 0035: Improve auto-update diagnostics and recovery

**Status:** Implemented

## Summary

Make update failures diagnosable and actionable for production users. Record updater events under `~/.karen/logs/`, report update state in plain, friendly language, keep raw diagnostics out of the UI, and preserve correct retry/restart behavior after partial success.

## Problem

`UpdatePrompt.vue` catches errors from both `downloadAndInstall()` and `relaunch()` in one block. Both become the same generic “update failed” message, while the actual error goes only to `console.warn`. The Tauri host does not register a logger, so production users have no reliable way to retrieve this message. A failed restart can also be reported as a failed installation even when installation already succeeded.

The current installation failure has a confirmed frontend cause: `found` uses Vue's deep `ref()` around Tauri's class-based `Update` resource. Vue proxies the instance. Calling `downloadAndInstall()` through that proxy makes Tauri's `Resource.rid` private-field accessor throw `TypeError: Cannot read private member from an object whose class did not declare it` before the updater IPC download command runs. A component-level repro and the running Tauri dev app both show this exact exception. The generic catch then hides the phase and cause from the user.

The current release manifest and macOS ARM64 archive are published and reachable; the archive signature verifies against the public key configured by this app. The running app is installed at `/Applications/Karen.app`, owned by the current user, and the updater implementation in Tauri 2.14 includes an administrator-authorization path for protected app bundles. These facts do not establish the cause of this particular failure; the product currently discards the evidence needed to tell.

## Goals

- Persist updater check, download, install, and restart outcomes under `~/.karen/logs/`, including the phase, app version, target platform, and underlying error.
- Show a short, localized, user-friendly explanation and a clear next action; never show raw logs, exception text, or log paths in the UI.
- Keep download progress visible while an update is being fetched.
- Show update phases in the existing update card: downloading, installing, and restarting.
- Separate installation success from restart failure so the UI never says installation failed after it succeeded; offer a clear restart action in that state.
- Preserve retry behavior after a recoverable check, download, signature, or install failure.
- Keep detailed diagnostics available in the local log for support and development without requiring developer tools.

## Non-goals

- Changing release signing keys, updater endpoints, release asset formats, or platform support.
- Automatic retries, background downloads, silent installation, or changing startup-check behavior.
- Uploading logs or diagnostics to a remote service.
- Showing raw diagnostics, file paths, or technical log details in the user interface.
- Exposing secrets, authorization headers, or full sensitive URLs in logs.

## Design

- Register Tauri's official logging plugin in `apps/desktop/src-tauri/src/main.rs` with a `Folder` target at `~/.karen/logs/`, rather than Tauri's platform-default log directory. Resolve the home directory using Tauri's path API and create the log directory through the logging plugin's supported setup. Tauri's v2 logging documentation confirms the plugin supports a custom folder target.
- In `apps/desktop/src/components/UpdatePrompt.vue`, hold the Tauri `Update` instance in a shallow ref so Vue preserves its class identity and private resource state. Model the update lifecycle as explicit phases: checking, available, downloading, installing, installed/restart-required, and failed. Show download progress in the existing update card using updater download events; do not infer download completion from elapsed time. Keep check errors distinct from install errors.
- Log phase transitions and full failure causes through the Tauri logger. Redact credentials, authorization headers, and sensitive URL query data before logging. Keep raw diagnostics out of the UI.
- Present a short localized message for each failure phase, with a clear action such as retrying the check/download or installing manually. Keep detailed diagnostics out of the card. For a restart failure after successful installation, say the update is installed and instruct the user to quit and reopen Karen.
- Treat `downloadAndInstall()` completion as installation success. Run `relaunch()` in a separate error path; if restart fails, leave the installed state intact and present a restart instruction instead of enabling a second install attempt.
- Add localized strings for phase labels, progress, each failure class, restart-required state, and diagnostic detail labels in all supported locales and locale schemas.
- Keep update orchestration in the frontend component and logging configuration in the Tauri shell. Do not add updater or filesystem business logic to Vue beyond this UI state.

## Acceptance

- A component regression test uses a class instance with a private field accessed by `downloadAndInstall()` and proves install reaches the updater call without Vue proxy errors.
- Component tests cover check failure; download progress and failure; installation failure; successful installation followed by relaunch failure; retry after recoverable failure; friendly phase-specific messages and actions; and no raw exception or log path in rendered UI.
- The existing update card visibly transitions through download, install, and restart states without opening a separate log or diagnostics view.
- Logging configuration writes updater diagnostics to `~/.karen/logs/` using Tauri's supported custom folder target; failure logging redacts sensitive data.
- Locale tests verify all new keys exist in every supported locale.
- Tauri host builds with the logging plugin and its least required capability; Rust lint and tests pass.
- `pnpm test`, `pnpm lint`, and `pnpm build` pass.
- Manual macOS acceptance: install an update from a production-like signed build, verify updater records land under `~/.karen/logs/`, and verify each forced failure state shows only friendly guidance and the correct next action.
