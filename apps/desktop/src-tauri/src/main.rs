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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_and_focus_main(app);
        }))
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![update_tray_status])
        .run(tauri::generate_context!())
        .expect("failed to run Karen desktop");
}
