mod network;
mod network_ffi;
mod state;

use std::sync::Mutex;

use tauri::{webview::PageLoadEvent, Manager};

use crate::state::State;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dns::init())
        .append_invoke_initialization_script("window.__cpe_shim_tauri_api_version__ = 0;")
        .invoke_handler(tauri::generate_handler![
            network::net_set_rpc_result_channel,
            network::net_open_session,
            network::net_close_session,
            network::net_dispatch_rpc,
        ])
        .setup(|app| {
            app.manage(Mutex::new(State::default()));

            network::init();
            Ok(())
        })
        .on_page_load(|_webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                network::net_close_all_sessions();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
