use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".local/share/mcshell")
}

pub fn profile_file() -> PathBuf {
    data_dir().join("profile.txt")
}

pub fn skin_file() -> PathBuf {
    data_dir().join("skin.png")
}