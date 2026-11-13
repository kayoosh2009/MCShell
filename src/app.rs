use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::profile::Profile;
use crate::{browse, fabric, launcher, mods, packs, skin, versions, window, worlds};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Profile,
    Versions,
    Mods,
    Textures,
    Skins,
    Worlds,
    Launch,
    More,
}

impl Tab {
    pub const ALL: [Tab; 8] = [Tab::Profile, Tab::Versions, Tab::Mods, Tab::Textures, Tab::Skins, Tab::Worlds, Tab::Launch, Tab::More];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Profile => "Profile",
            Tab::Versions => "Versions",
            Tab::Mods => "Mods",
            Tab::Textures => "Textures",
            Tab::Skins => "Skins",
            Tab::Worlds => "Worlds",
            Tab::Launch => "Launch",
            Tab::More => "More",
        }
    }

    pub fn index(&self) -> usize {
        Tab::ALL.iter().position(|t| t == self).unwrap()
    }
}

pub enum InputMode {
    Normal,
    EditingUsername,
    EditingVersion,
    ConfirmDelete,
}

pub enum DeleteTarget {
    Version(String),
    Mod(mods::ModEntry),
    Pack(String),
}

pub struct App {
    pub current_tab: Tab,
    pub should_quit: bool,
    pub status: String,
    pub progress_log: Vec<String>,
    pub profile: Profile,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub has_skin: bool,
    pub installed: Vec<String>,
    pub remote_versions: Vec<versions::VersionEntry>,
    pub list_index: usize,
    pub last_vanilla: Option<String>,
    pub mods: Vec<mods::ModEntry>,
    pub packs: Vec<String>,
    pub worlds: Vec<String>,
    pub browse_launch: Option<(Instant, String)>,
    browse: Option<browse::BrowseServer>,
    pending_delete: Option<DeleteTarget>,
    pub hide_after_launch: bool,
    pub show_logs_separate: bool,
    pub discord_on: bool,
    pub more_page: usize,
    pub crash: Option<crate::logs::Report>,
    pending_launch: Option<String>,
    progress_rx: Option<Receiver<String>>,
}

impl App {
    pub fn new() -> Self {
        let st = crate::settings::load();
        Self {
            current_tab: Tab::Profile,
            should_quit: false,
            status: "arrows/tab: switch tabs, q: quit".to_string(),
            progress_log: Vec::new(),
            profile: Profile::load(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            has_skin: skin::has_skin(),
            installed: versions::installed_versions(),
            remote_versions: Vec::new(),
            list_index: 0,
            last_vanilla: None,
            mods: mods::list(),
            packs: packs::list(),
            worlds: worlds::list(),
            browse_launch: None,
            browse: None,
            pending_delete: None,
            hide_after_launch: st.hide_after_launch,
            show_logs_separate: st.show_logs_separate,
            discord_on: st.discord,
            more_page: 0,
            crash: None,
            pending_launch: None,
            progress_rx: None,
        }
    }

    fn save_settings(&self) {
        crate::settings::save(&crate::settings::Settings {
            discord: self.discord_on,
            hide_after_launch: self.hide_after_launch,
            show_logs_separate: self.show_logs_separate,
        });
    }

    pub fn toggle_setting(&mut self, i: usize) {
        match i {
            0 => {
                self.discord_on = !self.discord_on;
                if self.discord_on {
                    crate::discord::init();
                } else {
                    crate::discord::shutdown();
                }
            }
            1 => self.hide_after_launch = !self.hide_after_launch,
            _ => self.show_logs_separate = !self.show_logs_separate,
        }
        self.save_settings();
    }

    fn more_len(&self) -> usize {
        match self.more_page {
            0 => 3,
            1 => crate::java::JAVAS.len(),
            _ => 0,
        }
    }

    pub fn spawn_java_install(&mut self, major: u32) {
        self.progress_log.clear();
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.status = format!("installing java {major}...");
        std::thread::spawn(move || {
            let progress = |msg: String| { let _ = tx.send(msg); };
            if let Err(e) = crate::java::install(major, &progress) {
                let _ = tx.send(format!("error: {e}"));
            }
        });
    }

    fn next_tab(&mut self) {
        let i = (self.current_tab.index() + 1) % Tab::ALL.len();
        self.current_tab = Tab::ALL[i];
        self.on_tab_change();
    }

    fn prev_tab(&mut self) {
        let len = Tab::ALL.len();
        let i = (self.current_tab.index() + len - 1) % len;
        self.current_tab = Tab::ALL[i];
        self.on_tab_change();
    }

    pub fn on_tab_change(&mut self) {
        self.list_index = 0;
        self.mods = mods::list();
        self.packs = packs::list();
        self.worlds = worlds::list();
    }

    pub fn spawn_version_install(&mut self, id: String) {
        self.progress_log.clear();
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.status = format!("installing {id}...");
        std::thread::spawn(move || {
            let progress = |msg: String| { let _ = tx.send(msg); };
            if let Err(e) = versions::install_version(&id, &progress) {
                let _ = tx.send(format!("error: {e}"));
            }
        });
    }

    pub fn spawn_fabric_install(&mut self, mc_version: String) {
        self.progress_log.clear();
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.status = format!("installing fabric for {mc_version}...");
        std::thread::spawn(move || {
            let progress = |msg: String| { let _ = tx.send(msg); };
            match fabric::fetch_loader_versions(&mc_version) {
                Ok(loaders) => match loaders.first() {
                    Some(loader) => {
                        if let Err(e) = fabric::install_fabric(&mc_version, loader, &progress) {
                            let _ = tx.send(format!("error: {e}"));
                        }
                    }
                    None => { let _ = tx.send("no fabric loader found".to_string()); }
                },
                Err(e) => { let _ = tx.send(format!("error: {e}")); }
            }
        });
    }

    pub fn poll_progress(&mut self) -> bool {
        let mut changed = false;
        let mut disconnected = false;
        if let Some(rx) = &self.progress_rx {
            loop {
                match rx.try_recv() {
                    Ok(msg) => {
                        changed = true;
                        self.progress_log.push(msg.clone());
                        if self.progress_log.len() > 6 {
                            self.progress_log.remove(0);
                        }
                        self.status = msg;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => { disconnected = true; break; }
                }
            }
        }
        if disconnected {
            changed = true;
            self.progress_rx = None;
            self.installed = versions::installed_versions();
            self.progress_log.push("✅ Install complete!".to_string());
        }
        changed
    }

    fn ask_delete(&mut self, target: DeleteTarget, label: &str) {
        self.pending_delete = Some(target);
        self.input_mode = InputMode::ConfirmDelete;
        self.status = format!("Delete {label}? (y/n)");
    }

    fn confirm_delete(&mut self) {
        let Some(target) = self.pending_delete.take() else { return };
        self.input_mode = InputMode::Normal;
        match target {
            DeleteTarget::Version(id) => match versions::delete_version(&id) {
                Ok(()) => {
                    self.status = format!("deleted {id}");
                    self.progress_log.clear();
                    self.installed = versions::installed_versions();
                    if self.list_index >= self.installed.len() && self.list_index > 0 {
                        self.list_index -= 1;
                    }
                }
                Err(e) => self.status = format!("delete error: {e}"),
            },
            DeleteTarget::Mod(m) => {
                self.status = match mods::remove(&m) {
                    Ok(()) => format!("removed {}", m.name),
                    Err(e) => format!("remove error: {e}"),
                };
                self.mods = mods::list();
                if self.list_index >= self.mods.len() && self.list_index > 0 {
                    self.list_index -= 1;
                }
            }
            DeleteTarget::Pack(name) => {
                self.status = match packs::remove(&name) {
                    Ok(()) => format!("removed {name}"),
                    Err(e) => format!("remove error: {e}"),
                };
                self.packs = packs::list();
                if self.list_index >= self.packs.len() && self.list_index > 0 {
                    self.list_index -= 1;
                }
            }
        }
    }

    pub fn start_browse(&mut self) {
        if self.browse.is_none() {
            match browse::start() {
                Ok(s) => self.browse = Some(s),
                Err(e) => {
                    self.status = format!("browser error: {e}");
                    return;
                }
            }
        }
        let kind = if self.current_tab == Tab::Textures { "resourcepack" } else { "mod" };
        if let Some(s) = &self.browse {
            self.browse_launch = Some((Instant::now(), s.url(kind)));
        }
    }

    pub fn poll_browse(&mut self) -> bool {
        let mut dirty = self.browse_launch.is_some();
        if let Some(s) = &self.browse {
            let mut changed = false;
            while let Ok(msg) = s.events.try_recv() {
                self.status = msg;
                changed = true;
                dirty = true;
            }
            if changed {
                self.mods = mods::list();
                self.packs = packs::list();
            }
        }
        let ready = self
            .browse_launch
            .as_ref()
            .filter(|(t, _)| t.elapsed() >= Duration::from_secs(3))
            .map(|(_, u)| u.clone());
        if let Some(url) = ready {
            window::open_url(&url);
            self.browse_launch = None;
            self.status = format!("browser: {url}");
            dirty = true;
        }
        dirty
    }

    pub fn launch_version(&mut self, id: String) {
        let uuid = self.profile.offline_uuid();
        let username = self.profile.username.clone();
        let win = if self.hide_after_launch { crate::window::hide_current() } else { Err(String::new()) };
        if let Err(e) = &win {
            if !e.is_empty() {
                self.status = format!("hide window failed: {e}");
            }
        }
        self.status = match launcher::launch(&id, &username, &uuid, self.show_logs_separate, win) {
            Ok(()) => format!("launched {id}"),
            Err(e) => format!("launch error: {e}"),
        };
    }

    pub fn fetch_remote_versions(&mut self) {
        match versions::fetch_manifest() {
            Ok(list) => {
                self.status = format!("fetched {} versions", list.len());
                self.remote_versions = list;
            }
            Err(e) => self.status = format!("fetch error: {e}"),
        }
    }

    pub fn delete_version(&mut self, id: &str) {
        match versions::delete_version(id) {
            Ok(()) => {
                self.status = format!("deleted {id}");
                self.progress_log.clear();
                self.installed = versions::installed_versions();
            }
            Err(e) => self.status = format!("delete error: {e}"),
        }
    }

    pub fn toggle_mod(&mut self, idx: usize) {
        if let Some(m) = self.mods.get(idx).cloned() {
            self.status = match mods::toggle(&m) {
                Ok(()) => format!("toggled {}", m.name),
                Err(e) => format!("toggle error: {e}"),
            };
            self.mods = mods::list();
        }
    }

    pub fn delete_mod(&mut self, idx: usize) {
        if let Some(m) = self.mods.get(idx).cloned() {
            self.status = match mods::remove(&m) {
                Ok(()) => format!("removed {}", m.name),
                Err(e) => format!("remove error: {e}"),
            };
            self.mods = mods::list();
        }
    }

    pub fn delete_pack(&mut self, idx: usize) {
        if let Some(name) = self.packs.get(idx).cloned() {
            self.status = match packs::remove(&name) {
                Ok(()) => format!("removed {name}"),
                Err(e) => format!("remove error: {e}"),
            };
            self.packs = packs::list();
        }
    }

    pub fn export_world(&mut self, idx: usize) {
        if let Some(name) = self.worlds.get(idx).cloned() {
            self.status = match worlds::export(&name) {
                Ok(p) => format!("exported to {}", p.display()),
                Err(e) => format!("export error: {e}"),
            };
        }
    }

    pub fn cancel_browse(&mut self) {
        self.browse_launch = None;
        self.status = "cancelled".to_string();
    }
}