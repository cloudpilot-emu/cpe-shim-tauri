#[cfg(dev)]
pub fn get_app_url() -> String {
    "http://localhost:4200".to_owned()
}

#[cfg(not(dev))]
pub fn get_app_url() -> String {
    "https://cloudpilot-emu.github.io/app-preview".to_owned()
}
