use std::fs;

use crate::paths;

pub const GITHUB: &str = "https://github.com/kayoosh2009/MCShell";
pub const EMAIL: &str = "PUT_YOUR_EMAIL";
pub const DISCORD: &str = "PUT_DISCORD_INVITE_LINK";
pub const TELEGRAM: &str = "PUT_TELEGRAM_CHANNEL_LINK";

#[derive(Clone, Copy)]
pub struct Settings {
    pub discord: bool,
    pub hide_after_launch: bool,
    pub show_logs_separate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { discord: true, hide_after_launch: false, show_logs_separate: true }
    }
}

pub fn load() -> Settings {
    let mut s = Settings::default();
    let text = fs::read_to_string(paths::settings_file()).unwrap_or_default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim() == "true";
        match k.trim() {
            "discord" => s.discord = v,
            "hide_after_launch" => s.hide_after_launch = v,
            "show_logs_separate" => s.show_logs_separate = v,
            _ => {}
        }
    }
    s
}

pub fn save(s: &Settings) {
    let _ = fs::create_dir_all(paths::data_dir());
    let text = format!(
        "discord={}\nhide_after_launch={}\nshow_logs_separate={}\n",
        s.discord, s.hide_after_launch, s.show_logs_separate
    );
    let _ = fs::write(paths::settings_file(), text);
}