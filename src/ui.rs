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
    
    // 1. Сначала просто получаем текст для любой вкладки
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
        Tab::Skins => format!(
            "Skin: {}\n\ndrop a PNG file on the terminal window to set it",
            if app.has_skin { "set" } else { "not set" }
        ),
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

    // 2. Специальная логика отрисовки только для вкладки Skins с загруженным скином
    if app.current_tab == Tab::Skins && app.has_skin {
        let inner_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        // Текст слева
        let body = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(body, inner_chunks[0]);

        // Превью скина справа
        if let Ok(img) = image::open(crate::paths::skin_file()) {
            let lines = crate::skin_view::skin_to_lines(&img, 32, 32);
            let preview = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Skin preview"));
            f.render_widget(preview, inner_chunks[1]);
        }
    } else {
        // Стандартная отрисовка для всех остальных вкладок (и для Skins без картинки)
        let body = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(body, chunks[1]);
    }

    // 3. Статус бар рисуется ВСЕГДА, независимо от вкладки
    f.render_widget(Paragraph::new(app.status.as_str()), chunks[2]);
}