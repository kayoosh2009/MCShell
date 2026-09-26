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
            _ => format!(
                "Username: {}\nUUID: {}\n\npress 'e' to edit username",
                app.profile.username,
                app.profile.offline_uuid()
            ),
        },
        Tab::Versions => {
            let mut lines = vec![match app.input_mode {
                InputMode::EditingVersion => format!("Version id: {}_", app.input_buffer),
                _ => "i: type version id, r: fetch list, up/down+enter: install, f: install fabric".to_string(),
            }];
            lines.push(format!("installed: {}", app.installed.join(", ")));
            lines.push(String::new());
            for (i, v) in app.remote_versions.iter().take(25).enumerate() {
                let marker = if i == app.list_index { ">" } else { " " };
                lines.push(format!("{marker} {} ({})", v.id, v.kind));
            }
            lines.join("\n")
        }
        Tab::Mods => "Mod list, toggle and remove will show here.".to_string(),
        Tab::Skins => {
            let text = format!(
                "Skin: {}\n\ndrop a PNG file on the terminal window to set it",
                if app.has_skin { "set" } else { "not set" }
            );
            
            let body = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(body, chunks[1]);
            
            if app.has_skin {
                if let Ok(img) = image::open(crate::paths::skin_file()) {
                    let lines = skin_view::skin_to_lines(&img, 32, 32);
                    let preview = Paragraph::new(lines)
                        .block(Block::default().borders(Borders::ALL).title("Skin preview"));
                    
                    // Размещаем превью в правой части экрана
                    let preview_area = ratatui::layout::Layout::default()
                        .direction(ratatui::layout::Direction::Horizontal)
                        .constraints([
                            ratatui::layout::Constraint::Percentage(50),
                            ratatui::layout::Constraint::Percentage(50),
                        ])
                        .split(chunks[1])[1];
                    
                    f.render_widget(preview, preview_area);
                }
            }
            
            return; // Пропускаем общий рендеринг body ниже
        },
        Tab::Worlds => "World list, export and import will show here.".to_string(),
        Tab::Launch => {
            let mut lines = vec!["up/down: select, enter: launch".to_string(), String::new()];
            for (i, v) in app.installed.iter().enumerate() {
                let marker = if i == app.list_index { ">" } else { " " };
                lines.push(format!("{marker} {v}"));
            }
            lines.join("\n")
        }
    };
    let body = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(body, chunks[1]);

    f.render_widget(Paragraph::new(app.status.as_str()), chunks[2]);
}