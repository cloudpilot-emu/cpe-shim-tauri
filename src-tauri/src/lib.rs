mod network;
mod network_ffi;

use tauri::webview::PageLoadEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .append_invoke_initialization_script("window.__cpe_shim_tauri_api_version__ = 0;")
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            network::net_open_session,
            network::net_close_session,
            network::net_dispatch_rpc,
        ])
        .setup(|app| {
            network::init(app.handle());
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
