mod loading_controller;
mod network;
mod network_ffi;
mod state;
mod store_keys;
mod url;
mod version;

use std::sync::Mutex;

use tauri::{webview::PageLoadEvent, Manager};
use tauri_plugin_store::StoreExt;

#[cfg(not(mobile))]
use tauri::WindowBuilder;

use crate::{loading_controller::LoadingController, state::State};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dns::init())
        .invoke_handler(tauri::generate_handler![
            network::net_set_rpc_result_channel,
            network::net_open_session,
            network::net_close_session,
            network::net_dispatch_rpc,
            loading_controller::set_service_worker_installed,
        ])
        .setup(|app| {
            app.store(store_keys::STORE_NAME)?;

            app.manage(Mutex::new(State::default()));
            app.manage(LoadingController::default());

            network::init();

            #[cfg(not(mobile))]
            WindowBuilder::new(app, "main")
                .inner_size(800., 600.)
                .build()?;

            app.state::<LoadingController>()
                .inner()
                .load(app.handle().clone())?;

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
