pub mod config;
pub mod gpu;
pub mod system_info;
pub mod window;

use std::collections::HashMap;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use config::ConfigStore;
use system_info::{RefreshOptions, SystemResources, SystemState};

/// Run blocking work (sysinfo, PDH, Win32 calls) off the async runtime threads.
async fn blocking<T, F>(app: AppHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppHandle) -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())
}

// ───────────────────────────── Commands ─────────────────────────────

#[tauri::command]
fn get_app_config(config: tauri::State<'_, ConfigStore>) -> HashMap<String, String> {
    config.snapshot()
}

/// Merge `values` into the persisted config (other keys are left untouched).
#[tauri::command]
fn update_app_config(
    config: tauri::State<'_, ConfigStore>,
    values: HashMap<String, String>,
) -> Result<(), String> {
    config.update(values)
}

#[tauri::command]
async fn get_system_resources(
    app: AppHandle,
    options: RefreshOptions,
) -> Result<SystemResources, String> {
    blocking(app, move |app| app.state::<SystemState>().sample(&options)).await
}

#[tauri::command]
async fn clear_ram_cache(app: AppHandle) -> Result<String, String> {
    blocking(app, |app| app.state::<SystemState>().clear_ram_cache()).await
}

/// Async on purpose: creating a window from a sync command can deadlock on Windows.
#[tauri::command]
async fn toggle_widget(app: AppHandle) -> Result<(), String> {
    window::toggle_widget(&app)
}

#[tauri::command]
async fn show_main_window(app: AppHandle) {
    window::show_main(&app);
}

// ───────────────────────────── App ─────────────────────────────

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let reset_item = MenuItem::with_id(app, "reset_pos", "위치 초기화 (&R)", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "show_main", "메인 창 열기 (&O)", true, None::<&str>)?;
    let widget_item =
        MenuItem::with_id(app, "toggle_widget", "미니 위젯 전환 (&W)", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "종료 (&X)", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&reset_item, &show_item, &widget_item, &sep, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip(format!("Sys4 v{}", app.package_info().version))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "reset_pos" => window::reset_active(app),
            "show_main" => window::show_main(app),
            "toggle_widget" => {
                let _ = window::toggle_widget(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::focus_active(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first so a second launch is intercepted before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::focus_active(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let config_path = app.path().app_config_dir().ok().map(|p| p.join("config.json"));
            app.manage(ConfigStore::load(config_path));
            app.manage(SystemState::new());

            build_tray(app)?;

            // The main window starts hidden (tauri.conf.json) so it can be placed at its saved,
            // on-screen position before the first paint instead of jumping after load.
            window::show_main(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            update_app_config,
            get_system_resources,
            clear_ram_cache,
            toggle_widget,
            show_main_window
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Closing the main window quits the app (the widget is only a companion view).
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } = event
            {
                if label == window::MAIN {
                    app.exit(0);
                }
            }
        });
}
