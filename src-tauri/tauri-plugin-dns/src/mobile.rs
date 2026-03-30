use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

pub struct Dns<R: Runtime>(pub PluginHandle<R>);

pub fn init<R: Runtime>(
    _app: &AppHandle<R>,
    api: PluginApi<R, ()>,
) -> Result<Dns<R>, Box<dyn std::error::Error>> {
    let handle = api.register_android_plugin("io.github.cloudpilotemu.plugins.dns", "DnsPlugin")?;
    Ok(Dns(handle))
}

impl<R: Runtime> Dns<R> {
    pub fn run_mobile_plugin<T: DeserializeOwned>(
        &self,
        method: &str,
        payload: impl serde::Serialize,
    ) -> Result<T, Box<dyn std::error::Error>> {
        self.0
            .run_mobile_plugin(method, payload)
            .map_err(Into::into)
    }
}
