use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::profile::Profile;
use crate::skin;
use crate::ui;

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
    pub const ALL: [Tab; 6] = [
        Tab::Profile,
        Tab::Versions,
        Tab::Mods,
        Tab::Skins,
        Tab::Worlds,
        Tab::Launch,
    ];

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
}

pub struct App {
    pub current_tab: Tab,
    pub should_quit: bool,
    pub status: String,
    pub profile: Profile,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub has_skin: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::Profile,
            should_quit: false,
            status: "arrows/tab: switch tabs, q: quit".to_string(),
            profile: Profile::load(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            has_skin: skin::has_skin(),
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
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
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
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key.code),
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}