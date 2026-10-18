# RFC 0021: Single-instance desktop application guard

**Status:** Implemented

## Summary

Enforce a single running instance of the Karen desktop application using `tauri-plugin-single-instance`. When a user attempts to launch a second instance of Karen (via Dock, Spotlight, Application launcher, or command line), the secondary process automatically signals the active primary instance and exits cleanly. The primary instance intercepts the event and brings its window to the foreground using the existing `show_and_focus_main` handler.

## Problem

Karen is a persistent system monitor featuring a live system tray icon, active background hardware sampling, and cached disk tree states. If a user inadvertently launches Karen while an existing instance is already running (e.g., hidden to the menu bar/system tray or minimized), a second independent instance is spawned.

This creates several issues:
1. **Duplicate System Tray Icons**: Multiple Karen status items appear in the macOS menu bar or OS tray, confusing the user.
2. **Resource Waste & Competing Sidecars**: Each running instance spawns its own unprivileged CLI sidecar (`karen --stream-json`), duplicating CPU/memory polling and socket table scans.
3. **Window State Ambiguity**: The user expects clicking Karen in the Dock or launcher to reveal the existing session, rather than creating a disconnected blank window.

## Goals

1. **Strict Single-Instance Enforcement**:
   - Integrate `tauri-plugin-single-instance` for Tauri 2 into `apps/desktop/src-tauri`.
   - Prevent secondary desktop application instances from spawning windows or running sidecars.
2. **Foreground Activation on Duplicate Launch**:
   - When a secondary instance starts, invoke `show_and_focus_main` on the active primary instance: unminimize, show (if hidden to tray), and set window focus.
3. **Zero Overhead & Clean Exit**:
   - The secondary process terminates immediately and cleanly without displaying errors or orphaned processes.
4. **Preserve Architectural Invariants**:
   - Tauri host remains a thin shell; standalone CLI usage (`karen [PATH]`) is unaffected and remains available for multiple simultaneous terminal sessions.
   - 100% test coverage and clean lints maintained.

## Non-goals

- Restricting multiple standalone terminal CLI invocations (`karen` command-line scanner).
- Inter-instance command-line argument passing beyond focusing the primary window.

## Design

### 1. Tauri Dependency Update (`apps/desktop/src-tauri/Cargo.toml`)

Add `tauri-plugin-single-instance`:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-shell = "2"
tauri-plugin-single-instance = "2"
```

### 2. Main Window Focus Hook (`apps/desktop/src-tauri/src/main.rs`)

Register the single-instance plugin before setup:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        show_and_focus_main(app);
    }))
    .plugin(tauri_plugin_shell::init())
    // ... setup and tray
```

When a second instance launches:
1. `tauri-plugin-single-instance` detects the existing named mutex/socket.
2. It sends the launch notification to the primary instance.
3. The callback executes `show_and_focus_main(app)`.
4. The secondary process exits before spawning any webview window or CLI sidecar.

## Acceptance

1. `tauri-plugin-single-instance` is registered in `apps/desktop/src-tauri`.
2. Running a second instance focuses and unminimizes the existing Karen desktop window.
3. `pnpm lint`, `pnpm test`, and `pnpm build` pass with 0 errors and 100% test coverage.
