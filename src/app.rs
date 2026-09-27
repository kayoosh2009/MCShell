use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::profile::Profile;
use crate::{fabric, launcher, skin, ui, versions};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Profile,
    Versions,
    Mods,
    Skins,
    Worlds,
    Launch,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Profile, Tab::Versions, Tab::Mods, Tab::Skins, Tab::Worlds, Tab::Launch];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Profile => "Profile",
            Tab::Versions => "Versions",
            Tab::Mods => "Mods",
            Tab::Skins => "Skins",
            Tab::Worlds => "Worlds",
            Tab::Launch => "Launch",
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
    progress_rx: Option<Receiver<String>>,
}

impl App {
    pub fn new() -> Self {
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
            launch_mode: launcher::LaunchMode::NewTerminal,
            pending_launch: None,
            progress_rx: None,
        }
    }

    fn next_tab(&mut self) {
        let i = (self.current_tab.index() + 1) % Tab::ALL.len();
        self.current_tab = Tab::ALL[i];
    }

    fn prev_tab(&mut self) {
        let len = Tab::ALL.len();
        let i = (self.current_tab.index() + len - 1) % len;
        self.current_tab = Tab::ALL[i];
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

    fn handle_key(&mut self, code: KeyCode) {
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
                    match versions::delete_version(&id) {
                        Ok(()) => {
                            self.status = format!("deleted {id}");
                            self.progress_log.clear();
                            self.installed = versions::installed_versions();
                            // Корректируем индекс, если удалили последний элемент
                            if self.list_index >= self.installed.len() && self.list_index > 0 {
                                self.list_index -= 1;
                            }
                        }
                        Err(e) => self.status = format!("delete error: {e}"),
                    }
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
                KeyCode::Char('1') if self.current_tab == Tab::Launch => {
                    self.launch_mode = launcher::LaunchMode::InPlace;
                }
                KeyCode::Char('2') if self.current_tab == Tab::Launch => {
                    self.launch_mode = launcher::LaunchMode::NewTerminal;
                }
                KeyCode::Enter if self.current_tab == Tab::Launch => {
                    if let Some(id) = self.installed.get(self.list_index).cloned() {
                        self.pending_launch = Some(id);
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
        }
    }

    fn handle_paste(&mut self, text: String) {
        if self.current_tab != Tab::Skins {
            return;
        }
        let path = PathBuf::from(text.trim());
        match skin::set_skin(&path) {
            Ok(dest) => {
                self.has_skin = true;
                self.status = format!("skin saved: {}", dest.display());
            }
            Err(e) => self.status = format!("skin error: {e}"),
        }
    }
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    loop {
        app.poll_progress();
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key.code),
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }

        if let Some(id) = app.pending_launch.take() {
            let uuid = app.profile.offline_uuid();
            let username = app.profile.username.clone();
            if app.launch_mode == launcher::LaunchMode::InPlace {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
                let result = launcher::launch(&id, &username, &uuid, launcher::LaunchMode::InPlace);
                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnterAlternateScreen, EnableBracketedPaste)?;
                terminal.clear()?;
                app.status = match result {
                    Ok(()) => format!("launched {id}"),
                    Err(e) => format!("launch error: {e}"),
                };
            } else {
                app.status = match launcher::launch(&id, &username, &uuid, launcher::LaunchMode::NewTerminal) {
                    Ok(()) => format!("launched {id} in new terminal"),
                    Err(e) => format!("launch error: {e}"),
                };
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}