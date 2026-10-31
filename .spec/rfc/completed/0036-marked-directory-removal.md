# RFC 0036: Make marked-directory removal failures actionable

**Status:** Implemented

## Summary

Keep eligible marked-directory removal behavior and existing progress feedback. Pass a stable failure category from the removal backend through the CLI so the desktop can show a localized reason and recovery action while preserving failed marks for retry.

## Problem

The user reports that a marked directory cannot be deleted and that the UI gives little help. A real CLI run on a nested temporary directory succeeds in both permanent and macOS Trash modes. A controlled permission-denied run correctly leaves the directory in place and emits `Permission denied (os error 13)`. The current UI displays that raw OS error without a clear recovery step. The user's original target and failure cause remain unknown, so this RFC does not assume that all deletion failures share one cause.

## Goals

- Preserve successful permanent and Trash removal for eligible nested directories.
- Preserve safety refusals and explain them without exposing raw OS errors as the primary message.
- Send a stable failure category with each failed removal item.
- Show a concise localized explanation and next action for permission-denied and other removal failures.
- Keep failed marks available after returning to the map so users can retry.
- Preserve the existing progress workspace, determinate progress, completion summary, and return lock while removal runs.

## Non-goals

- Weakening root, mount, symlink, home, or system-path protections.
- Adding privilege escalation or changing file-removal semantics.
- Rebuilding the existing progress workspace or changing updater behavior.
- Claiming a root cause for the user's original directory before reproducing it.

## Design

- Keep filesystem operations and error classification in `monitor-disk`; do not add filesystem access or removal logic to Vue or the Tauri shell.
- Classify failures as `safety_guard`, `permission_denied`, `not_found`, or `other`. Classify safety refusals where the guard rejects the target; classify I/O failures by `ErrorKind`, without parsing OS error text.
- Add `error_code` to each `removal_item` event. Keep the existing `error` detail field unchanged for compatibility and support; successful items send null error fields.
- Map permission denial and other known categories to complete messages with a useful next action. Unknown errors use a safe generic message; raw OS text is not the headline.
- Render path, localized message, and recovery action in the existing review workspace. Keep progress, successful removal, retry marks, and the return lock unchanged.
- Cover permission-denied and not-found CLI events, preservation of the raw diagnostic and retry mark, category mapping, and UI behavior with tests. Do not add a claimed backend regression for ordinary directory removal because current permanent and Trash reproductions both succeed.

## Acceptance

- A marked nested directory is removed in permanent mode and moved to Trash in macOS Trash mode; completion reports success and the target disappears from the scanned tree.
- A deterministic permission-denied removal emits a stable category, reports failure, and leaves the directory and mark available for retry.
- The UI renders localized, actionable guidance for permission-denied and unknown failures without presenting the raw OS error as primary copy.
- Existing root, mount-point, symlink, home, and system-tree safety tests continue to refuse unsafe targets.
- Existing progress UI reports completed/total and prevents returning to the map until removal completes; completion shows removed/failed counts and reclaimed space.
- `pnpm test`, `pnpm lint`, and `pnpm build` pass. Relevant Rust tests and lint pass.
