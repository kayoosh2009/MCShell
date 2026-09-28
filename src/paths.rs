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

pub fn private_key_file() -> PathBuf {
    data_dir().join("skin_key.pem")
}

pub fn public_key_pem_file() -> PathBuf {
    data_dir().join("skin_key_pub.pem")
}

pub fn mods_dir() -> PathBuf {
    game_dir().join("mods")
}

pub fn saves_dir() -> PathBuf {
    game_dir().join("saves")
}

pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}

pub fn clean_path(text: &str) -> PathBuf {
    let t = text.trim().trim_matches(|c: char| c == '\'' || c == '"');
    let t = t.strip_prefix("file://").unwrap_or(t);
    PathBuf::from(t.replace("\\ ", " "))
}

pub fn packs_dir() -> PathBuf {
    game_dir().join("resourcepacks")
}