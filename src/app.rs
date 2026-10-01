use std::io;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::profile::Profile;
use crate::{browse, fabric, launcher, mods, packs, paths, skin, ui, versions, window, worlds};

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

    fn toggle_setting(&mut self, i: usize) {
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

    fn spawn_java_install(&mut self, major: u32) {
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

    fn on_tab_change(&mut self) {
        self.list_index = 0;
        self.mods = mods::list();
        self.packs = packs::list();
        self.worlds = worlds::list();
    }

    fn spawn_version_install(&mut self, id: String) {
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

    fn spawn_fabric_install(&mut self, mc_version: String) {
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

    fn poll_progress(&mut self) {
        let mut disconnected = false;
        if let Some(rx) = &self.progress_rx {
            loop {
                match rx.try_recv() {
                    Ok(msg) => {
                        self.progress_log.push(msg.clone());
                        // Храним только последние 6 сообщений, чтобы не забивать экран
                        if self.progress_log.len() > 6 {
                            self.progress_log.remove(0);
                        }
                        self.status = msg; // для нижней строки статуса
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => { disconnected = true; break; }
                }
            }
        }
        if disconnected {
            self.progress_rx = None;
            self.installed = versions::installed_versions();
            self.progress_log.push("✅ Install complete!".to_string());
        }
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

    fn start_browse(&mut self) {
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

    fn poll_browse(&mut self) {
        if let Some(s) = &self.browse {
            let mut changed = false;
            while let Ok(msg) = s.events.try_recv() {
                self.status = msg;
                changed = true;
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
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        if self.crash.is_some() {
            if matches!(code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter) {
                self.crash = None;
                return;
            }
            if let Some(r) = self.crash.as_mut() {
                match code {
                    KeyCode::Up => r.scroll = r.scroll.saturating_sub(1),
                    KeyCode::Down => r.scroll = (r.scroll + 1).min(r.tail.len().saturating_sub(1)),
                    KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(15),
                    KeyCode::PageDown => r.scroll = (r.scroll + 15).min(r.tail.len().saturating_sub(1)),
                    KeyCode::Char('o') => window::open_folder(&paths::game_dir().join("logs")),
                    KeyCode::Char('c') => {
                        if let Some(p) = &r.crash_file {
                            window::open_url(p);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if self.browse_launch.is_some() {
            self.browse_launch = None;
            self.status = "cancelled".to_string();
            return;
        }
        match self.input_mode {
            InputMode::Normal => match code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Right | KeyCode::Tab => self.next_tab(),
                KeyCode::Left | KeyCode::BackTab => self.prev_tab(),
                KeyCode::Char('e') if self.current_tab == Tab::Profile => {
                    self.input_mode = InputMode::EditingUsername;
                    self.input_buffer = self.profile.username.clone();
                }
                KeyCode::Char('i') if self.current_tab == Tab::Versions => {
                    self.input_mode = InputMode::EditingVersion;
                    self.input_buffer = String::new();
                }
                KeyCode::Char('r') if self.current_tab == Tab::Versions => match versions::fetch_manifest() {
                    Ok(list) => {
                        self.status = format!("fetched {} versions", list.len());
                        self.remote_versions = list;
                        self.list_index = 0;
                    }
                    Err(e) => self.status = format!("fetch error: {e}"),
                },
                KeyCode::Char('f') if self.current_tab == Tab::Versions => {
                    match self.last_vanilla.clone() {
                        Some(mc) => self.spawn_fabric_install(mc),
                        None => self.status = "install a vanilla version first".to_string(),
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete if self.current_tab == Tab::Launch && !self.installed.is_empty() => {
                    let id = self.installed[self.list_index].clone();
                    self.ask_delete(DeleteTarget::Version(id.clone()), &id);
                }
                KeyCode::Down if self.current_tab == Tab::Versions && !self.remote_versions.is_empty() => {
                    self.list_index = (self.list_index + 1) % self.remote_versions.len();
                }
                KeyCode::Up if self.current_tab == Tab::Versions && !self.remote_versions.is_empty() => {
                    self.list_index = (self.list_index + self.remote_versions.len() - 1) % self.remote_versions.len();
                }
                KeyCode::Enter if self.current_tab == Tab::Versions => {
                    if let Some(v) = self.remote_versions.get(self.list_index) {
                        let id = v.id.clone();
                        self.last_vanilla = Some(id.clone());
                        self.spawn_version_install(id);
                    }
                }
                KeyCode::Down if self.current_tab == Tab::Launch && !self.installed.is_empty() => {
                    self.list_index = (self.list_index + 1) % self.installed.len();
                }
                KeyCode::Up if self.current_tab == Tab::Launch && !self.installed.is_empty() => {
                    self.list_index = (self.list_index + self.installed.len() - 1) % self.installed.len();
                }
                KeyCode::Char('1') if self.current_tab == Tab::Launch => self.toggle_setting(1),
                KeyCode::Char('2') if self.current_tab == Tab::Launch => self.toggle_setting(2),
                KeyCode::Enter if self.current_tab == Tab::Launch => {
                    if let Some(id) = self.installed.get(self.list_index).cloned() {
                        self.pending_launch = Some(id);
                    }
                }
                KeyCode::Down if self.current_tab == Tab::Mods && !self.mods.is_empty() => {
                    self.list_index = (self.list_index + 1) % self.mods.len();
                }
                KeyCode::Up if self.current_tab == Tab::Mods && !self.mods.is_empty() => {
                    self.list_index = (self.list_index + self.mods.len() - 1) % self.mods.len();
                }
                KeyCode::Char(' ') if self.current_tab == Tab::Mods => {
                    if let Some(m) = self.mods.get(self.list_index).cloned() {
                        self.status = match mods::toggle(&m) {
                            Ok(()) => format!("toggled {}", m.name),
                            Err(e) => format!("toggle error: {e}"),
                        };
                        self.mods = mods::list();
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete if self.current_tab == Tab::Mods => {
                    if let Some(m) = self.mods.get(self.list_index).cloned() {
                        let label = m.name.clone();
                        self.ask_delete(DeleteTarget::Mod(m), &label);
                    }
                }
                KeyCode::Char('o') if self.current_tab == Tab::Mods => {
                    window::open_folder(&paths::mods_dir());
                }
                KeyCode::Char('b') if matches!(self.current_tab, Tab::Mods | Tab::Textures) => {
                    self.start_browse();
                }
                KeyCode::Down if self.current_tab == Tab::Worlds && !self.worlds.is_empty() => {
                    self.list_index = (self.list_index + 1) % self.worlds.len();
                }
                KeyCode::Up if self.current_tab == Tab::Worlds && !self.worlds.is_empty() => {
                    self.list_index = (self.list_index + self.worlds.len() - 1) % self.worlds.len();
                }
                KeyCode::Char('e') if self.current_tab == Tab::Worlds => {
                    if let Some(name) = self.worlds.get(self.list_index).cloned() {
                        self.status = match worlds::export(&name) {
                            Ok(p) => format!("exported to {}", p.display()),
                            Err(e) => format!("export error: {e}"),
                        };
                    }
                }
                KeyCode::Char('o') if self.current_tab == Tab::Worlds => {
                    window::open_folder(&paths::saves_dir());
                }
                KeyCode::Down if self.current_tab == Tab::Textures && !self.packs.is_empty() => {
                    self.list_index = (self.list_index + 1) % self.packs.len();
                }
                KeyCode::Up if self.current_tab == Tab::Textures && !self.packs.is_empty() => {
                    self.list_index = (self.list_index + self.packs.len() - 1) % self.packs.len();
                }
                KeyCode::Char('d') | KeyCode::Delete if self.current_tab == Tab::Textures => {
                    if let Some(name) = self.packs.get(self.list_index).cloned() {
                        self.ask_delete(DeleteTarget::Pack(name.clone()), &name);
                    }
                }
                KeyCode::Char('o') if self.current_tab == Tab::Textures => {
                    window::open_folder(&paths::packs_dir());
                }
                KeyCode::Char(c @ '1'..='3') if self.current_tab == Tab::More => {
                    self.more_page = c as usize - '1' as usize;
                    self.list_index = 0;
                }
                KeyCode::Down if self.current_tab == Tab::More && self.more_len() > 0 => {
                    self.list_index = (self.list_index + 1) % self.more_len();
                }
                KeyCode::Up if self.current_tab == Tab::More && self.more_len() > 0 => {
                    self.list_index = (self.list_index + self.more_len() - 1) % self.more_len();
                }
                KeyCode::Enter | KeyCode::Char(' ') if self.current_tab == Tab::More && self.more_page == 0 => {
                    self.toggle_setting(self.list_index);
                }
                KeyCode::Enter if self.current_tab == Tab::More && self.more_page == 1 => {
                    let major = crate::java::JAVAS[self.list_index].0;
                    self.spawn_java_install(major);
                }
                KeyCode::Char('d') | KeyCode::Delete if self.current_tab == Tab::More && self.more_page == 1 => {
                    let major = crate::java::JAVAS[self.list_index].0;
                    self.status = match crate::java::remove(major) {
                        Ok(()) => format!("java {major} removed"),
                        Err(e) => format!("remove error: {e}"),
                    };
                }
                KeyCode::Char(c @ ('g' | 'm' | 'd' | 't')) if self.current_tab == Tab::More && self.more_page == 2 => {
                    use crate::settings::{DISCORD, EMAIL, GITHUB, TELEGRAM};
                    match c {
                        'g' => window::open_url(GITHUB),
                        'm' => window::open_url(&format!("mailto:{EMAIL}")),
                        'd' => window::open_url(DISCORD),
                        _ => window::open_url(TELEGRAM),
                    }
                }
                _ => {}
            },
            InputMode::EditingUsername => match code {
                KeyCode::Enter => {
                    let name = self.input_buffer.trim().to_string();
                    if !name.is_empty() {
                        self.profile.set_username(name);
                        self.status = "username saved".to_string();
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => { self.input_buffer.pop(); }
                KeyCode::Char(c) => self.input_buffer.push(c),
                _ => {}
            },
            InputMode::EditingVersion => match code {
                KeyCode::Enter => {
                    let id = self.input_buffer.trim().to_string();
                    if !id.is_empty() {
                        self.last_vanilla = Some(id.clone());
                        self.spawn_version_install(id);
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => { self.input_buffer.pop(); }
                KeyCode::Char(c) => self.input_buffer.push(c),
                _ => {}
            },
            InputMode::ConfirmDelete => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete(),
                _ => {
                    self.pending_delete = None;
                    self.input_mode = InputMode::Normal;
                    self.status = "cancelled".to_string();
                }
            },
        }
    }

    fn handle_paste(&mut self, text: String) {
        let path = paths::clean_path(&text);
        match self.current_tab {
            Tab::Skins => match skin::set_skin(&path) {
                Ok(dest) => {
                    self.has_skin = true;
                    self.status = format!("skin saved: {}", dest.display());
                }
                Err(e) => self.status = format!("skin error: {e}"),
            },
            Tab::Mods => {
                self.status = match mods::add(&path) {
                    Ok(name) => format!("added {name}"),
                    Err(e) => format!("add error: {e}"),
                };
                self.mods = mods::list();
            }
            Tab::Textures => {
                self.status = match packs::add(&path) {
                    Ok(name) => format!("added {name}"),
                    Err(e) => format!("add error: {e}"),
                };
                self.packs = packs::list();
            }
            Tab::Worlds => {
                self.status = match worlds::import(&path) {
                    Ok(name) => format!("imported world {name}"),
                    Err(e) => format!("import error: {e}"),
                };
                self.worlds = worlds::list();
            }
            _ => {}
        }
    }
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    loop {
        app.poll_progress();
        app.poll_browse();
        if let Some(r) = crate::logs::take_pending() {
            app.crash = Some(r);
        }
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Конвертируем раскладку только в обычном режиме.
                    // В режимах редактирования (имя, версия) оставляем как есть, 
                    // чтобы пользователь мог нормально печатать кириллицей.
                    let code = match key.code {
                        KeyCode::Char(c) if !matches!(app.input_mode, InputMode::EditingUsername | InputMode::EditingVersion) => {
                            KeyCode::Char(crate::keymap::to_latin(c))
                        }
                        other => other,
                    };
                    app.handle_key(code);
                }
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }

        if let Some(id) = app.pending_launch.take() {
            let uuid = app.profile.offline_uuid();
            let username = app.profile.username.clone();
            let win = if app.hide_after_launch { crate::window::hide_current() } else { Err(String::new()) };
            if let Err(e) = &win {
                if !e.is_empty() {
                    app.status = format!("hide window failed: {e}");
                }
            }

            if app.show_logs_separate {
                app.status = match launcher::launch(&id, &username, &uuid, true, win) {
                    Ok(()) => format!("launched {id} in new terminal"),
                    Err(e) => format!("launch error: {e}"),
                };
            } else {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
                let result = launcher::launch(&id, &username, &uuid, false, win);
                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnterAlternateScreen, EnableBracketedPaste)?;
                terminal.clear()?;
                app.status = match result {
                    Ok(()) => format!("launched {id}"),
                    Err(e) => format!("launch error: {e}"),
                };
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}