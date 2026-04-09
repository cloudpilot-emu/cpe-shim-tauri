use std::{
    ops::DerefMut,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use tauri::{async_runtime, AppHandle, Listener, Manager, Url, WebviewUrl};
use tauri_plugin_store::StoreExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[cfg(not(mobile))]
use uuid::Uuid;

#[cfg(not(mobile))]
use tauri::{utils::config::BackgroundThrottlingPolicy, LogicalPosition, WebviewBuilder, Window};

#[cfg(mobile)]
use tauri::WebviewWindowBuilder;

use crate::{loading_controller, store_keys, url::get_app_url, version::VERSION};

#[cfg(not(mobile))]
const LABEL_SPLASH: &str = "splash";
const LABEL_APP: &str = "app";

#[cfg(not(mobile))]
const SPLASHSCREEN_URL: &str = "/splashscreen/index.html";

#[cfg(mobile)]
const INLINE_HTML_SPLASH: &str = include_str!("inline_splash.phtml");

const TIMEOUT_SECONDS: u64 = 10;

struct LoadGuard(Arc<Mutex<bool>>);

impl Drop for LoadGuard {
    fn drop(&mut self) {
        *self.0.lock().unwrap().deref_mut() = false;
    }
}

impl LoadGuard {
    fn new(is_locked: Arc<Mutex<bool>>) -> Self {
        assert!(
            !*is_locked.lock().unwrap(),
            "attempt to load while webview is already loading"
        );

        Self(is_locked)
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

        show_splash_view(&app)?;

        let challenge = Uuid::new_v4().to_string();
        let app_url = get_app_url();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        add_app_view(&app, Url::from_str(&app_url)?, challenge.as_str())?;

        listen_for_handshake(&app, challenge.clone(), tx);

        wait_for_load(&app, rx, app_url, lock);

        Ok(())
    }

    #[cfg(mobile)]
    pub fn load(&self, app: AppHandle) -> anyhow::Result<()> {
        let lock = LoadGuard::new(self.is_loading.clone());

        let challenge = "cpe";
        let app_url = get_app_url();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        initialize_app_view(&app, Url::from_str(&app_url)?, challenge)?;

        listen_for_handshake(&app, challenge.into(), tx);

        wait_for_load(&app, rx, app_url, lock);

        Ok(())
    }
}

#[tauri::command]
pub fn set_service_worker_installed(app: AppHandle, worker_installed: bool) {
    let store = app.store(store_keys::STORE_NAME).unwrap();

    store.set(store_keys::KEY_WORKER_INSTALLED, worker_installed);
}

#[tauri::command]
pub fn reload(app: AppHandle) {
    let loading_controller = app.state::<LoadingController>();

    loading_controller
        .load(app.clone())
        .expect("failed to reload");
}

fn listen_for_handshake(app: &AppHandle, challenge: String, tx: UnboundedSender<()>) {
    app.once("handshake", move |event| {
        if let Ok(challenge_from_event) = serde_json::from_str::<String>(event.payload()) {
            if challenge_from_event == challenge {
                tx.send(()).unwrap();
            } else {
                println!(
                    "received bad challenge from webview {}",
                    challenge_from_event
                );
            }
        } else {
            println!("handshake with invalid payload {}", event.payload());
        }
    });
}

fn wait_for_load(app: &AppHandle, mut rx: UnboundedReceiver<()>, url: String, lock: LoadGuard) {
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

        handle_handshake(&app, lock);
    });
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
fn handle_handshake(app: &AppHandle, lock: LoadGuard) {
    show_app_view(&app);
    drop(lock);
    let _ = remove_view(&app, LABEL_SPLASH);
}

#[cfg(not(mobile))]
fn get_window(app: &AppHandle) -> anyhow::Result<Window> {
    app.get_window("main")
        .ok_or(anyhow::format_err!("failed to retrieve window"))
}

#[cfg(not(mobile))]
fn remove_view(app: &AppHandle, label: &str) -> anyhow::Result<()> {
    let window = get_window(app)?;

    if let Some(view) = window.get_webview(label) {
        view.close()?;
    }

    Ok(())
}

#[cfg(not(mobile))]
fn show_splash_view(app: &AppHandle) -> anyhow::Result<()> {
    remove_view(app, LABEL_SPLASH)?;

    let window = get_window(app)?;

    let builder = WebviewBuilder::new(
        LABEL_SPLASH,
        tauri::WebviewUrl::App(SPLASHSCREEN_URL.into()),
    )
    .auto_resize();

    #[cfg(not(dev))]
    let builder = builder.devtools(false);

    let _ = window.add_child(builder, LogicalPosition::new(0, 0), window.inner_size()?)?;

    Ok(())
}

#[cfg(not(mobile))]
fn add_app_view(app: &AppHandle, url: Url, challenge: &str) -> anyhow::Result<()> {
    remove_view(app, LABEL_APP)?;

    let window = get_window(app)?;

    let builder = WebviewBuilder::new(LABEL_APP, WebviewUrl::External(url))
        .background_throttling(BackgroundThrottlingPolicy::Disabled)
        .disable_drag_drop_handler()
        .initialization_script(format!("
            (function() {{
                window.__cpe_shim_tauri_version = {};
                window.__cpe_shim_tauri_challenge = '{}';
                
                if (sessionStorage.getItem('TAURI_APP_FIRST_LOAD') === null) {{
                    sessionStorage.setItem('TAURI_APP_FIRST_LOAD', '1');
                    return;
                }}

                if (!!window.navigator.serviceWorker?.controller) {{
                    setTimeout(() => __TAURI__.event.emit('handshake', window.__cpe_shim_tauri_challenge), 5000);
                }}
            }})()
        ", VERSION, challenge))
        .auto_resize();

    #[cfg(not(dev))]
    let builder = builder.devtools(false);

    let app_view = window.add_child(builder, LogicalPosition::new(0, 0), window.inner_size()?)?;
    app_view.hide()?;

    Ok(())
}

#[cfg(not(mobile))]
fn show_app_view(app: &AppHandle) {
    if let Some(app_view) = app.get_webview(LABEL_APP) {
        let _ = app_view.show();
    }
}

#[cfg(mobile)]
fn initialize_app_view(app: &AppHandle, url: Url, challenge: &str) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL_APP) {
        window.navigate(url)?;
    } else {
        let builder = WebviewWindowBuilder::new(app, LABEL_APP, WebviewUrl::External(url))
            .disable_drag_drop_handler()
            .auto_resize()
            .initialization_script(format!(
                "
                    (function() {{                        
                        window.__cpe_shim_tauri_version = {};
                        window.__cpe_shim_tauri_challenge = '{}';

                        let hasConnectionIssue = false;

                        if (sessionStorage.getItem('TAURI_APP_FIRST_LOAD') === null) {{
                            sessionStorage.setItem('TAURI_APP_FIRST_LOAD', '1');
                        }} else {{
                            hasConnectionIssue = true;

                            if (!!window.navigator.serviceWorker?.controller) {{
                                setTimeout(() => __TAURI__.event.emit('handshake', window.__cpe_shim_tauri_challenge), 5000);
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

                        const splashDelay = new Promise(r => setTimeout(r, 500));

                        __TAURI__.event.listen('handshake', () => splashDelay.then(() => {{
                            const splashElement = document.getElementById('inline-splash');

                            if (splashElement) document.body.removeChild(splashElement);
                            sessionStorage.removeItem('TAURI_APP_FIRST_LOAD');
                        }}));
                    }})()
                ",
                VERSION,
                challenge,
                serde_json::to_string(INLINE_HTML_SPLASH)?
            ));

        #[cfg(not(dev))]
        let builder = builder.devtools(true);

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
fn handle_handshake(_app: &AppHandle, lock: LoadGuard) {
    // Nothing to do, we remove the splashscreen directly in JS
    drop(lock);
}
