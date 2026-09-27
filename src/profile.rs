use std::fs;

use md5::{Digest, Md5};

use crate::paths;

pub struct Profile {
    pub username: String,
}

impl Profile {
    pub fn load() -> Self {
        let username = fs::read_to_string(paths::profile_file())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Player".to_string());
        Self { username }
    }

    pub fn save(&self) {
        let _ = fs::create_dir_all(paths::data_dir());
        let _ = fs::write(paths::profile_file(), &self.username);
    }

    pub fn set_username(&mut self, name: String) {
        self.username = name;
        self.save();
    }

    pub fn offline_uuid(&self) -> String {
        offline_uuid(&self.username)
    }
}

pub fn offline_uuid(username: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{username}"));
    let mut digest = hasher.finalize();
    digest[6] = (digest[6] & 0x0F) | 0x30;
    digest[8] = (digest[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3],
        digest[4], digest[5],
        digest[6], digest[7],
        digest[8], digest[9],
        digest[10], digest[11], digest[12], digest[13], digest[14], digest[15]
    )
}

pub fn strip_dashes(uuid: &str) -> String {
    uuid.chars().filter(|c| *c != '-').collect()
}