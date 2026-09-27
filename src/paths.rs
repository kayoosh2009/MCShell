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

pub fn game_dir() -> PathBuf {
    data_dir().join("game")
}

pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}

pub fn libraries_dir() -> PathBuf {
    data_dir().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}

pub fn authlib_injector_jar() -> PathBuf {
    data_dir().join("authlib-injector.jar")
}