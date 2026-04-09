use crate::app_channel::AppChannel;

pub const STORE_NAME: &str = "store.json";

pub const KEY_WORKER_INSTALLED_PREVIEW: &str = "workerInstalledPreview";
pub const KEY_WORKER_INSTALLED_STABLE: &str = "workerInstalledStable";
pub const KEY_APP_CHANNEL: &str = "appChannel";

pub fn key_worker_installed(channel: AppChannel) -> &'static str {
    match channel {
        AppChannel::Preview => KEY_WORKER_INSTALLED_PREVIEW,
        AppChannel::Stable => KEY_WORKER_INSTALLED_STABLE,
    }
}
