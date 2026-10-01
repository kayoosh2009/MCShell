mod app;
mod fabric;
mod launcher;
mod paths;
mod profile;
mod skin;
mod ui;
mod versions;
mod skin_view;
mod skin_server;
mod window;
mod art;
mod browse;
mod mods;
mod packs;
mod worlds;
mod discord;
mod stats;
mod java;
mod settings;
mod keymap;

use std::io;

use anyhow::Result;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if settings::load().discord {
        discord::init();
    }
    stats::init();
    let result = app::run(&mut terminal);
    discord::shutdown();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
    terminal.show_cursor()?;

    result
}