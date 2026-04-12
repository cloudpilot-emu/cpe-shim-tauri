use std::{
    ops::DerefMut,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde_json::{Number, Value};
use tauri::{async_runtime, AppHandle, Listener, Manager, Url, WebviewUrl};
use tauri_plugin_store::StoreExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[cfg(not(mobile))]
use tauri::{utils::config::BackgroundThrottlingPolicy, LogicalPosition, WebviewBuilder, Window};

#[cfg(mobile)]
use tauri::WebviewWindowBuilder;

use crate::{
    app_channel::AppChannel,
    store_keys::{self, key_worker_installed, KEY_APP_CHANNEL},
    url::get_app_url,
};

#[cfg(not(mobile))]
const LABEL_SPLASH: &str = "splash";
const LABEL_APP: &str = "app";

#[cfg(not(mobile))]
const SPLASHSCREEN_URL: &str = "/splashscreen/index.html";

const TIMEOUT_SECONDS: u64 = 10;
const SPLASH_SCREEN_MIN_TTL_MSEC: u64 = 500;
const FALLBACK_HANDSHAKE_AFTER_MSEC: u64 = 5000;

struct LoadGuard(Arc<Mutex<bool>>);

impl Drop for LoadGuard {
    fn drop(&mut self) {
        *self.0.lock().unwrap().deref_mut() = false;
    }
}

impl LoadGuard {
    fn new(is_locked_mut: Arc<Mutex<bool>>) -> Self {
        {
            let mut is_locked = is_locked_mut.lock().unwrap();

            assert!(
                !*is_locked,
                "attempt to load while webview is already loading"
            );

            *is_locked = true;
        }

        Self(is_locked_mut)
    }
}

#[derive(Default)]
pub struct LoadingController {
    is_loading: Arc<Mutex<bool>>,
}

impl LoadingController {
    #[cfg(not(mobile))]
    pub fn load(&self, app: AppHandle) -> anyhow::Result<()> {
        let lock = LoadGuard::new(self.is_loading.clone());
        let load_start_at = Instant::now();

        show_splash_view(&app)?;

        let channel = get_app_channel(app.clone());
        let app_url = get_app_url(channel);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        println!("loading from channel: {}", channel);

        prepare_app_view(&app, Url::from_str(&app_url)?)?;

        listen_for_handshake(&app, tx);

        wait_for_load(&app, rx, app_url, load_start_at, lock);

        Ok(())
    }

    #[cfg(mobile)]
    pub fn load(&self, app: AppHandle) -> anyhow::Result<()> {
        let lock = LoadGuard::new(self.is_loading.clone());
        let load_start_at = Instant::now();

        let channel = get_app_channel(app.clone());
        let app_url = get_app_url(channel);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        initialize_app_view(&app, Url::from_str(&app_url)?)?;

        listen_for_handshake(&app, tx);

        wait_for_load(&app, rx, app_url, load_start_at, lock);

        Ok(())
    }
}

#[tauri::command]
pub fn set_service_worker_installed(app: AppHandle, worker_installed: bool) {
    let store = app.store(store_keys::STORE_NAME).unwrap();

    let channel = get_app_channel(app.clone());
    store.set(key_worker_installed(channel), worker_installed);
}

#[tauri::command]
pub fn reload(app: AppHandle) {
    let loading_controller = app.state::<LoadingController>();

    loading_controller
        .load(app.clone())
        .expect("failed to reload");
}

#[tauri::command]
pub fn get_app_channel(app: AppHandle) -> AppChannel {
    let store = app.store(store_keys::STORE_NAME).unwrap();

    store
        .get(KEY_APP_CHANNEL)
        .as_ref()
        .and_then(Value::as_number)
        .and_then(Number::as_i64)
        .and_then(AppChannel::from_number)
        .unwrap_or(AppChannel::Stable)
}

#[tauri::command]
pub fn switch_app_channel(app: AppHandle, channel: AppChannel) {
    let store = app.store(store_keys::STORE_NAME).unwrap();

    store.set(KEY_APP_CHANNEL, channel as i64);

    reload(app.clone());
}

fn listen_for_handshake(app: &AppHandle, tx: UnboundedSender<()>) {
    app.once("handshake", move |_| {
        tx.send(()).unwrap();
    });
}

fn wait_for_load(
    app: &AppHandle,
    mut rx: UnboundedReceiver<()>,
    url: String,
    load_start_at: Instant,
    lock: LoadGuard,
) {
    let app = app.clone();

    async_runtime::spawn(async move {
        loop {
            match tokio::time::timeout(Duration::from_secs(TIMEOUT_SECONDS), rx.recv()).await {
                Ok(Some(())) => break,
                Ok(None) => panic!("unreachable: channel closed"),
                Err(_) => {
                    println!("app failed to load within timeout");

                    handle_handshake_timeout(&app, &url);
                }
            }
        }

        handle_handshake(&app, load_start_at, lock).await;
    });
}

fn get_version(app: &AppHandle) -> u32 {
    let semver = &app.package_info().version;

    ((semver.major << 16) | (semver.minor << 8) | semver.patch) as u32
}

#[cfg(not(mobile))]
fn handle_handshake_timeout(app: &AppHandle, url: &str) {
    if let Some(splash_view) = app.get_webview(LABEL_SPLASH) {
        let _ = splash_view.eval("document.documentElement.classList.add('connection-issue');");
    }

    if let Some(app_view) = app.get_webview(LABEL_APP) {
        let _ = app_view.navigate(Url::from_str(url).unwrap());
    }
}

#[cfg(not(mobile))]
async fn handle_handshake(app: &AppHandle, load_start_at: Instant, lock: LoadGuard) {
    let time_since_load = Instant::now().duration_since(load_start_at);

    if (time_since_load.as_millis() as u64) < SPLASH_SCREEN_MIN_TTL_MSEC {
        tokio::time::sleep(
            Duration::from_millis(SPLASH_SCREEN_MIN_TTL_MSEC).saturating_sub(time_since_load),
        )
        .await;
    }

    if let Ok(window) = get_window(app) {
        if let Some(app_view) = window.get_webview(LABEL_APP) {
            let _ = app_view.show();
            let _ = app_view.eval("sessionStorage.removeItem('TAURI_APP_FIRST_LOAD');");
        }

        if let Some(splash_view) = window.get_webview(LABEL_SPLASH) {
            let _ = splash_view.hide();
            let _ =
                splash_view.eval("document.documentElement.classList.remove('connection-issue');");
        }
    }

    drop(lock);
}

#[cfg(not(mobile))]
fn get_window(app: &AppHandle) -> anyhow::Result<Window> {
    app.get_window("main")
        .ok_or(anyhow::format_err!("failed to retrieve window"))
}

#[cfg(not(dev))]
fn enable_dev_tools() -> bool {
    if let Some(value) = option_env!("TAURI_DEV_TOOLS") {
        !value.is_empty()
    } else {
        false
    }
}

#[cfg(not(mobile))]
fn show_splash_view(app: &AppHandle) -> anyhow::Result<()> {
    let window = get_window(app)?;

    if let Some(app_view) = window.get_webview(LABEL_APP) {
        app_view.hide()?;
    }

    if let Some(splash_view) = window.get_webview(LABEL_SPLASH) {
        splash_view.show()?;
    } else {
        let builder = WebviewBuilder::new(
            LABEL_SPLASH,
            tauri::WebviewUrl::App(SPLASHSCREEN_URL.into()),
        )
        .auto_resize();

        #[cfg(not(dev))]
        let builder = builder.devtools(enable_dev_tools());

        let _ = window.add_child(builder, LogicalPosition::new(0, 0), window.inner_size()?)?;
    }

    Ok(())
}

#[cfg(not(mobile))]
fn prepare_app_view(app: &AppHandle, url: Url) -> anyhow::Result<()> {
    let window = get_window(app)?;

    if let Some(app_view) = window.get_webview(LABEL_APP) {
        app_view.hide()?;
        app_view.navigate(url)?;

        return Ok(());
    }

    let builder = WebviewBuilder::new(LABEL_APP, WebviewUrl::External(url))
        .background_throttling(BackgroundThrottlingPolicy::Disabled)
        .disable_drag_drop_handler()
        .initialization_script(format!("
            (function() {{
                window.__cpe_shim_tauri_version = {};
                window.__cpe_shim_tauri_challenge = 'cpe';
                
                if (sessionStorage.getItem('TAURI_APP_FIRST_LOAD') === null) {{
                    sessionStorage.setItem('TAURI_APP_FIRST_LOAD', '1');
                    return;
                }}

                if (!!window.navigator.serviceWorker?.controller) {{
                    setTimeout(() => __TAURI__.event.emit('handshake', window.__cpe_shim_tauri_challenge), {});
                }}
            }})()
        ", get_version(app), FALLBACK_HANDSHAKE_AFTER_MSEC))
        .auto_resize();

    #[cfg(not(dev))]
    let builder = builder.devtools(enable_dev_tools());

    let app_view = window.add_child(builder, LogicalPosition::new(0, 0), window.inner_size()?)?;
    app_view.hide()?;

    Ok(())
}

#[cfg(mobile)]
fn initialize_app_view(app: &AppHandle, url: Url) -> anyhow::Result<()> {
    const INLINE_HTML_SPLASH: &str = include_str!("inline_splash.phtml");

    if let Some(window) = app.get_webview_window(LABEL_APP) {
        window.navigate(url)?;
    } else {
        let builder = WebviewWindowBuilder::new(app, LABEL_APP, WebviewUrl::External(url))
            .disable_drag_drop_handler()
            .auto_resize()
            .initialization_script(format!(
                "
                    (function() {{
                        window.__TAURI__.app.onBackButtonPress(() => undefined);
                                                
                        window.__cpe_shim_tauri_version = {};
                        window.__cpe_shim_tauri_challenge = 'cpe';

                        let hasConnectionIssue = false;

                        if (sessionStorage.getItem('TAURI_APP_FIRST_LOAD') === null) {{
                            sessionStorage.setItem('TAURI_APP_FIRST_LOAD', '1');
                        }} else {{
                            hasConnectionIssue = true;

                            if (!!window.navigator.serviceWorker?.controller) {{
                                setTimeout(() => __TAURI__.event.emit('handshake', window.__cpe_shim_tauri_challenge), {});
                            }}
                        }}

                        const html = {};

                        document.addEventListener('DOMContentLoaded', () => {{
                            const splashElement = document.createElement('div');
                            
                            splashElement.id = 'inline-splash';
                            splashElement.innerHTML = html;
                            if (hasConnectionIssue) splashElement.classList.add('connection-issue');

                            document.body.appendChild(splashElement);
                        }});

                        const splashDelay = new Promise(r => setTimeout(r, {}));

                        __TAURI__.event.listen('handshake', () => splashDelay.then(() => {{
                            const splashElement = document.getElementById('inline-splash');

                            if (splashElement) document.body.removeChild(splashElement);
                            sessionStorage.removeItem('TAURI_APP_FIRST_LOAD');
                        }}));
                    }})()
                ",
                get_version(app),
                FALLBACK_HANDSHAKE_AFTER_MSEC,
                serde_json::to_string(INLINE_HTML_SPLASH)?,
                SPLASH_SCREEN_MIN_TTL_MSEC
            ));

        #[cfg(not(dev))]
        let builder = builder.devtools(enable_dev_tools());

        builder.build()?;
    }

    Ok(())
}

#[cfg(mobile)]
fn handle_handshake_timeout(app: &AppHandle, url: &str) {
    if let Some(window) = app.get_webview_window(LABEL_APP) {
        println!("reload!");
        let _ = window.navigate(Url::from_str(url).unwrap());
    } else {
        println!("reload failed!");
    }
}

#[cfg(mobile)]
async fn handle_handshake(_app: &AppHandle, _load_start_at: Instant, lock: LoadGuard) {
    // Nothing to do, we remove the splashscreen directly in JS
    drop(lock);
}
