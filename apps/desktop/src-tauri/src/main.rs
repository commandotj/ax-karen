#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

struct TrayState {
    status_item: MenuItem<tauri::Wry>,
}

#[tauri::command]
fn update_tray_status(
    app: tauri::AppHandle,
    state: tauri::State<'_, TrayState>,
    title: Option<String>,
    summary: String,
) {
    let label = if summary.trim().is_empty() {
        "Karen — System Monitor".to_owned()
    } else {
        summary
    };
    let _ = state.status_item.set_text(&label);
    if let Some(tray) = app.tray_by_id("karen-tray") {
        let _ = tray.set_tooltip(Some(&label));
        if let Some(ref t) = title {
            let _ = tray.set_title(Some(t));
        }
    }
}

fn show_and_focus_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        let minimized = window.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = window.hide();
        } else {
            show_and_focus_main(app);
        }
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(
        app,
        "tray-status",
        "Karen — System Monitor",
        false,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let toggle_item = MenuItem::with_id(
        app,
        "toggle-window",
        "Show / Hide Karen",
        true,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let tab_runtime =
        MenuItem::with_id(app, "tab-runtime", "Open Runtime", true, None::<&str>)?;
    let tab_cpu = MenuItem::with_id(app, "tab-cpu", "Open CPU", true, None::<&str>)?;
    let tab_memory =
        MenuItem::with_id(app, "tab-memory", "Open Memory", true, None::<&str>)?;
    let tab_battery =
        MenuItem::with_id(app, "tab-battery", "Open Battery", true, None::<&str>)?;
    let tab_network =
        MenuItem::with_id(app, "tab-network", "Open Network", true, None::<&str>)?;
    let tab_space =
        MenuItem::with_id(app, "tab-space", "Open Space", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit_item =
        MenuItem::with_id(app, "quit", "Quit Karen", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &sep1,
            &toggle_item,
            &sep2,
            &tab_runtime,
            &tab_cpu,
            &tab_memory,
            &tab_battery,
            &tab_network,
            &tab_space,
            &sep3,
            &quit_item,
        ],
    )?;

    app.manage(TrayState { status_item });

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
        .ok()
        .or_else(|| app.default_window_icon().cloned());

    let mut builder = TrayIconBuilder::with_id("karen-tray")
        .menu(&menu)
        .tooltip("Karen — System Monitor")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle-window" => toggle_main_window(app),
            "tab-runtime" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "runtime");
            }
            "tab-cpu" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "cpu");
            }
            "tab-memory" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "memory");
            }
            "tab-battery" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "battery");
            }
            "tab-network" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "network");
            }
            "tab-space" => {
                show_and_focus_main(app);
                let _ = app.emit("tray-navigate", "space");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_and_focus_main(tray.app_handle());
            }
        });

    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }

    let _ = builder.build(app)?;
    Ok(())
}

/// Matches `minWidth` and `minHeight` in tauri.conf.json; a smaller monitor wins.
const MIN_WINDOW: (f64, f64) = (760.0, 600.0);

fn three_quarters(width: f64, height: f64) -> (f64, f64) {
    (
        (width * 0.75).max(MIN_WINDOW.0).min(width),
        (height * 0.75).max(MIN_WINDOW.1).min(height),
    )
}

/// Top-left corner that centers `size` on a monitor, horizontally and
/// vertically. Computed here because `Window::center` can run before a resize
/// has taken effect and then centers the old size.
fn centered(
    origin: (f64, f64),
    monitor: (f64, f64),
    size: (f64, f64),
) -> (f64, f64) {
    (
        origin.0 + (monitor.0 - size.0) / 2.0,
        origin.1 + (monitor.1 - size.1) / 2.0,
    )
}

/// Sizes the window from the monitor it opens on; the configured size stays
/// when the monitor cannot be read.
fn fit_main_window(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let screen = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);
    let size = three_quarters(screen.width, screen.height);
    let (x, y) = centered(
        (origin.x, origin.y),
        (screen.width, screen.height),
        size,
    );
    let _ = window.set_size(tauri::LogicalSize::new(size.0, size.1));
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_and_focus_main(app);
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            setup_tray(app)?;
            fit_main_window(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![update_tray_status])
        .run(tauri::generate_context!())
        .expect("failed to run Karen desktop");
}

#[cfg(test)]
mod tests {
    use super::{centered, three_quarters};

    #[test]
    fn window_is_three_quarters_of_the_monitor() {
        assert_eq!(three_quarters(1920.0, 1080.0), (1440.0, 810.0));
        assert_eq!(three_quarters(2560.0, 1440.0), (1920.0, 1080.0));
    }

    #[test]
    fn window_never_drops_below_the_configured_minimum() {
        assert_eq!(three_quarters(800.0, 600.0), (760.0, 600.0));
    }

    #[test]
    fn window_is_centered_on_its_monitor() {
        assert_eq!(
            centered((0.0, 0.0), (1920.0, 1080.0), (1440.0, 810.0)),
            (240.0, 135.0)
        );
        assert_eq!(
            centered((1920.0, -200.0), (2560.0, 1440.0), (1920.0, 1080.0)),
            (2240.0, -20.0)
        );
    }

    #[test]
    fn window_never_exceeds_the_monitor() {
        assert_eq!(three_quarters(700.0, 500.0), (700.0, 500.0));
    }
}
