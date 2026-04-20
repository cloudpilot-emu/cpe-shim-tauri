pub enum Platform {
    Unknown = 0,
    MacOS = 1,
    Windows = 2,
    Linux = 3,
    Android = 4,
}

impl Platform {
    pub fn get() -> Self {
        match std::env::consts::OS {
            "macos" => Self::MacOS,
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            "android" => Self::Android,
            _ => Self::Unknown,
        }
    }
}
