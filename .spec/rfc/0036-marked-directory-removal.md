# RFC 0036: Fix marked directory removal

**Status:** Draft

## Summary

Fix the full marked-folder removal path so an eligible folder selected in the disk map can be reviewed and removed, while protected paths remain blocked and any failed removal is clearly reported.

## Problem

The user reports that a marked folder cannot be deleted and the UI gives no useful help. Source inspection confirms directory nodes can be marked and the removal backend has recursive directory removal, so this is not an intentional file-only limitation. The failure could occur while marking, building the review plan, dispatching removal, or executing the OS-specific removal operation. Existing source alone does not identify which stage fails for the reported folder.

## Goals

- Reproduce and fix the failure for an ordinary, removable folder beneath the scanned root.
- Keep trash and permanent removal behavior correct for eligible folders with nested files and folders.
- Preserve path, mount, home, and system-tree guards.
- Always tell the user whether the folder was blocked, removed, or failed to remove, and provide a useful next action.
- Show visible removal progress immediately after final confirmation, including completed versus total targets and a progress indicator until completion.

## Non-goals

- Weakening root, mount-point, symlink, home, or system-path protections.
- Changing file-removal behavior or adding privilege escalation.
- Changing updater behavior or general review/navigation flows.

## Design

- Trace one directory from the treemap action through `DiskTreeView.vue`, `App.vue`, the `toggle_mark` and `confirm_remove` CLI protocol, `DiskMarks`, removal planning, and the platform removal backend.
- Establish a deterministic failing regression at the boundary that reproduces the reported symptom before changing implementation. Keep the test at the layer where the folder is lost, blocked incorrectly, or fails to remove.
- Make the smallest fix in the owning module. If the folder is rejected by a safety guard, keep the guard and expose its reason. If dispatch, OS removal, or completion handling is broken, correct that path without changing unrelated removal semantics.
- Ensure the review and completion states preserve enough information to distinguish blocked targets, failed targets, and successful removal. Show a localized, actionable explanation for each outcome instead of leaving the user with no guidance, then refresh the disk map after completion.
- Transition into a progress state immediately after final confirmation. Show a progress bar and completed/total target count in the existing review workspace; prevent returning to the map until the operation completes.
- After completion, show removed and failed counts plus reclaimed space. Explain each failed target in plain language with a next action. Preserve failed marks for a later retry; clear successful marks.

## Acceptance

- A regression test fails before the fix and passes after it for a marked, eligible directory containing nested contents.
- macOS acceptance covers both moving the directory to Trash and permanent removal.
- Tests confirm root, mount-point, symlink, home, and system-tree protections still refuse unsafe targets.
- A deliberately blocked or failed directory shows a clear, localized reason and next action; successful removal shows confirmation, clears the mark, and refreshes the map.
- Confirming removal immediately shows a progress view with completed/total count and progress indicator; completion replaces it with removed/failed counts and reclaimed space.
- Failed marks remain available for retry after returning to the map; successful marks do not return.
- `pnpm test`, `pnpm lint`, and `pnpm build` pass; relevant Rust crate tests and lint pass.
