mod commands;
mod monitor;
mod platform;
mod proxy;
mod subscription;

use std::sync::Arc;
use parking_lot::Mutex;

pub struct AppState {
    pub is_connected: Mutex<bool>,
    pub current_node: Mutex<Option<String>>,
    pub proxy_port: Mutex<u16>,
    pub singbox_process: Mutex<Option<std::process::Child>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("game_pp=debug,tauri=info")),
        )
        .init();

    let app_state = Arc::new(AppState {
        is_connected: Mutex::new(false),
        current_node: Mutex::new(None),
        proxy_port: Mutex::new(1080),
        singbox_process: Mutex::new(None),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::fetch_subscription,
            commands::parse_nodes,
            commands::test_latency,
            commands::start_proxy,
            commands::stop_proxy,
            commands::get_connection_status,
            commands::set_system_proxy,
            commands::get_running_apps,
            commands::get_app_state_json,
            commands::find_game_process,
            commands::get_process_ips,
            commands::scan_installed_games,
            commands::load_persistent_config,
            commands::save_persistent_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
