use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::app::{App, InputMode, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(Span::raw(t.title()))).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("MCShell"))
        .select(app.current_tab.index())
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    let title = app.current_tab.title();
    let text = match app.current_tab {
        Tab::Profile => match app.input_mode {
            InputMode::EditingUsername => format!("New username: {}_", app.input_buffer),
            InputMode::Normal => format!(
                "Username: {}\nUUID: {}\n\npress 'e' to edit username",
                app.profile.username,
                app.profile.offline_uuid()
            ),
        },
        Tab::Versions => "Installed versions and download will show here.".to_string(),
        Tab::Mods => "Mod list, toggle and remove will show here.".to_string(),
        Tab::Skins => format!(
            "Skin: {}\n\ndrop a PNG file on the terminal window to set it",
            if app.has_skin { "set" } else { "not set" }
        ),
        Tab::Worlds => "World list, export and import will show here.".to_string(),
        Tab::Launch => "Launch button and game log will show here.".to_string(),
    };
    let body = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(body, chunks[1]);

    f.render_widget(Paragraph::new(app.status.as_str()), chunks[2]);
}