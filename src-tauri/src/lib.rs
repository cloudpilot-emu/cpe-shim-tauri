mod app_channel;
mod loading_controller;
mod network;
mod network_ffi;
mod state;
mod store_keys;
mod url;
mod version;

use std::sync::Mutex;

use tauri::{
    ipc::RuntimeCapability,
    utils::acl::capability::{Capability, CapabilityFile, CapabilityRemote},
    webview::PageLoadEvent,
    Manager,
};
use tauri_plugin_store::StoreExt;

#[cfg(not(mobile))]
use tauri::WindowBuilder;

use crate::{
    app_channel::AppChannel, loading_controller::LoadingController, state::State, url::get_app_url,
};

struct CapabilityWrapper(Capability);

impl RuntimeCapability for CapabilityWrapper {
    fn build(self) -> CapabilityFile {
        CapabilityFile::Capability(self.0)
    }
}

#[cfg(dev)]
pub fn add_dev_capabilities(app: &mut tauri::App) -> anyhow::Result<()> {
    const DEFAULT_CAPABILITY: &str = include_str!("../capabilities/default.json");

    let mut capability = serde_json::from_str::<Capability>(DEFAULT_CAPABILITY)?;

    capability.identifier = "dev".into();
    capability.remote = Some(CapabilityRemote {
        urls: vec![
            get_app_url(AppChannel::Preview),
            get_app_url(AppChannel::Stable),
        ],
    });

    app.add_capability(CapabilityWrapper(capability))?;

    Ok(())
}

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
            loading_controller::reload,
            loading_controller::get_app_channel,
            loading_controller::switch_app_channel
        ])
        .setup(|app| {
            app.store(store_keys::STORE_NAME)?;

            app.manage(Mutex::new(State::default()));
            app.manage(LoadingController::default());

            network::init();

            #[cfg(dev)]
            add_dev_capabilities(app)?;

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
