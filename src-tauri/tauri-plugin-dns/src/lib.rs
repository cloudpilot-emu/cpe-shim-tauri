use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Runtime,
};

#[cfg(target_os = "android")]
use tauri::Manager;

#[cfg(target_os = "android")]
mod mobile;

#[cfg(target_os = "android")]
use mobile::Dns;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("dns")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                let handle = mobile::init(_app, _api)?;
                _app.manage(handle);
            }
            Ok(())
        })
        .build()
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct DnsResponse {
    primary: u32,
    secondary: u32,
}

/// Returns DNS servers as (primary, secondary) in network byte order,
/// or None on non-Android platforms.
pub fn get_dns_servers<R: Runtime>(app: &AppHandle<R>) -> Option<(u32, u32)> {
    #[cfg(target_os = "android")]
    {
        let dns = app.state::<Dns<R>>();

        let res: Result<DnsResponse, _> = dns.run_mobile_plugin("getDnsServers", ());
        if let Err(err) = res {
            println!("failed to query DNS from Android {}", err);
            return None;
        }

        let response = res.unwrap();

        if response.primary == 0 && response.secondary == 0 {
            None
        } else {
            Some((response.primary, response.secondary))
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        None
    }
}
