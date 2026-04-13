use crate::app_channel::AppChannel;

#[cfg(dev)]
pub fn get_app_url(channel: AppChannel) -> String {
    format!(
        "{}?{}",
        option_env!("TAURI_DEV_URL")
            .unwrap_or("http://localhost:4200")
            .to_owned(),
        channel.to_string()
    )
}

#[cfg(not(dev))]
pub fn get_app_url(channel: AppChannel) -> String {
    match channel {
        AppChannel::Preview => "https://cloudpilot-emu.github.io/app-preview".to_owned(),
        AppChannel::Stable => "https://cloudpilot-emu.github.io/app".to_owned(),
    }
}
