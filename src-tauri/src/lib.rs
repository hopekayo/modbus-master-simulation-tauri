mod commands;
mod models;
mod modbus;

use tauri::Manager;
use tokio::sync::{Mutex, RwLock};

use crate::models::MasterStatus;
use crate::modbus::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            context: Mutex::new(None),
            app_handle: Mutex::new(None),
            status: RwLock::new(MasterStatus {
                connected: false,
                mode: String::new(),
                details: "Disconnected".to_string(),
            }),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = handle.try_state::<AppState>() {
                    let mut app_handle = state.app_handle.lock().await;
                    *app_handle = Some(handle.clone());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_serial_ports,
            commands::connect_master,
            commands::disconnect_master,
            commands::read_master,
            commands::write_master,
            commands::scan_slaves,
            commands::get_master_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
