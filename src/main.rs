use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::{Frame, Terminal};

/// Вкладки главного меню. Порядок важен — по нему листаем стрелками/Tab.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Profiles,
    Versions,
    Mods,
    Skins,
    Worlds,
    Launch,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Profiles,
        Tab::Versions,
        Tab::Mods,
        Tab::Skins,
        Tab::Worlds,
        Tab::Launch,
    ];

    fn title(&self) -> &'static str {
        match self {
            Tab::Profiles => "Профили",
            Tab::Versions => "Версии",
            Tab::Mods => "Моды",
            Tab::Skins => "Скины",
            Tab::Worlds => "Миры",
            Tab::Launch => "Запуск",
        }
    }

    fn index(&self) -> usize {
        Tab::ALL.iter().position(|t| t == self).unwrap()
    }
}

struct App {
    current_tab: Tab,
    should_quit: bool,
    status: String,
}

impl App {
    fn new() -> Self {
        Self {
            current_tab: Tab::Profiles,
            should_quit: false,
            status: "Стрелки/Tab — вкладки, q или Esc — выход".to_string(),
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
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(app, key.code);
                }
                Event::Paste(text) => {
                    handle_paste(app, text);
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Right | KeyCode::Tab => app.next_tab(),
        KeyCode::Left | KeyCode::BackTab => app.prev_tab(),
        _ => {}
    }
}

fn handle_paste(app: &mut App, text: String) {
    let path = text.trim();
    app.status = format!("Получен путь: {path}");
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // вкладки
            Constraint::Min(0),    // содержимое
            Constraint::Length(1), // статус-строка
        ])
        .split(f.size());

    // --- Вкладки ---
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(Span::raw(t.title())))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("MCShell"))
        .select(app.current_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    let body_title = app.current_tab.title();
    let body_text = match app.current_tab {
        Tab::Profiles => "Здесь будет список offline-профилей (ник + uuid).",
        Tab::Versions => "Здесь будет список установленных версий и скачивание новых.",
        Tab::Mods => "Здесь будет список модов: вкл/выкл, удаление, добавление по пути.",
        Tab::Skins => "Здесь будет библиотека скинов.",
        Tab::Worlds => "Здесь будет список миров: экспорт/импорт.",
        Tab::Launch => "Здесь будет кнопка запуска и лог игры.",
    };
    let body = Paragraph::new(body_text)
        .block(Block::default().borders(Borders::ALL).title(body_title));
    f.render_widget(body, chunks[1]);

    let status = Paragraph::new(app.status.as_str());
    f.render_widget(status, chunks[2]);
}