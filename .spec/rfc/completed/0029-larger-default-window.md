# RFC 0029: Open the window at three quarters of the screen (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Open the main window at 3/4 of the monitor's width and 3/4 of its height, centered, instead of the fixed 1180x820.

## Problem

`tauri.conf.json` fixes the window at 1180x820 pixels. That is cramped for the Disk Tree treemap and its inspector on a large display and oversized on a small one.

## Goals

- On launch the main window is 75% of the current monitor's logical width and height, centered.
- The size never drops below the configured minimum (760x600) and never exceeds the monitor, so the window always fits on screen.
- If the monitor cannot be read, the window keeps the configured 1180x820.

## Non-goals

- No persistence of a user-chosen size or position across launches.
- No change to window decorations, the macOS overlay title bar, tray behavior or the frontend.
- Not reapplied on later show/hide; only on startup, so a user's manual resize in a session is kept.

## Design

- `apps/desktop/src-tauri/src/main.rs`: a pure function `three_quarters(width, height)` returns the logical size, floored at the minimum. In `setup`, a small step reads `current_monitor()`, converts its size to logical pixels with the scale factor, calls `set_size` then `center` on the `main` window, and does nothing if the monitor is unavailable. Sizing is window geometry only, so the shell stays free of monitor crates, filesystem access and business logic (invariant 10).
- `tauri.conf.json` keeps 1180x820 as the fallback and as the first-paint size.

## Acceptance

- Unit test: `three_quarters` returns 75% of a 1920x1080 and a 2560x1440 monitor, and floors a small monitor at 760x600, and caps a monitor smaller than that minimum at its own size.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`, `pnpm build` and `pnpm lint` pass.
- Manual: a fresh launch opens at about three quarters of the screen, centered, and the minimum size still holds. Multi-monitor placement is not verified.
