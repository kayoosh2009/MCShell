use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::app::{App, InputMode, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    if let Some((start, _)) = &app.browse_launch {
        draw_browse_splash(f, start.elapsed().as_secs());
        return;
    }

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
            _ => {
                use crate::stats::{fmt_ago, fmt_bytes, fmt_duration};
                let s = crate::stats::snapshot();
                let biggest = match &s.biggest_world {
                    Some((name, size)) => format!("{name} ({})", fmt_bytes(*size)),
                    None => "-".to_string(),
                };
                format!(
                    "Username: {}\nUUID: {}\n\npress 'e' to edit username\n\n\
                     --- Stats ---\n\
                     Playtime:       {}\n\
                     Launches:       {}\n\
                     Last played:    {}\n\n\
                     Worlds:         {} ({})\n\
                     Biggest world:  {}\n\
                     Mods:           {} ({})\n\
                     Texture packs:  {} ({})\n\
                     Launcher data:  {}",
                    app.profile.username,
                    app.profile.offline_uuid(),
                    fmt_duration(s.total_secs),
                    s.launches,
                    fmt_ago(s.last_played),
                    s.worlds,
                    fmt_bytes(s.worlds_bytes),
                    biggest,
                    s.mods,
                    fmt_bytes(s.mods_bytes),
                    s.packs,
                    fmt_bytes(s.packs_bytes),
                    fmt_bytes(s.data_bytes),
                )
            }
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
        Tab::Mods => {
            let mut lines = vec![
                "drop a .jar on the window to add".to_string(),
                "up/down: select  space: on/off  d: delete  o: open folder  b: browse online".to_string(),
                String::new(),
            ];
            if app.mods.is_empty() {
                lines.push("no mods yet".to_string());
            }
            for (i, m) in app.mods.iter().enumerate() {
                let marker = if i == app.list_index { ">" } else { " " };
                let check = if m.enabled { "[x]" } else { "[ ]" };
                lines.push(format!("{marker} {check} {}", m.name));
            }
            lines.join("\n")
        }
        Tab::Skins => {
            // Сначала получаем текст
            let text = format!(
                "Skin: {}\n\ndrop a PNG file on the terminal window to set it",
                if app.has_skin { "set" } else { "not set" }
            );
            
            // Если есть скин — делим экран
            if app.has_skin {
                let inner = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(chunks[1]);
                
                // Текст слева
                f.render_widget(
                    Paragraph::new(text.clone()).block(Block::default().borders(Borders::ALL).title(title)),
                    inner[0]
                );
                
                // Превью справа
                if let Ok(img) = image::open(crate::paths::skin_file()) {
                    let lines = crate::skin_view::skin_to_lines(&img, 32, 32);
                    f.render_widget(
                        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Skin preview")),
                        inner[1]
                    );
                }
                return; // выходим, чтобы не рисовать текст дважды
            }
            
            text  // возвращаем текст для обычного рендеринга (если нет скина)
        },
        Tab::Textures => {
            let mut lines = vec![
                "drop a .zip on the window to add".to_string(),
                "up/down: select  d: delete  o: open folder  b: browse online".to_string(),
                "enable packs in game: Options > Resource Packs".to_string(),
                String::new(),
            ];
            if app.packs.is_empty() {
                lines.push("no texture packs yet".to_string());
            }
            for (i, p) in app.packs.iter().enumerate() {
                let marker = if i == app.list_index { ">" } else { " " };
                lines.push(format!("{marker} {p}"));
            }
            lines.join("\n")
        }
        Tab::Worlds => {
            let mut lines = vec![
                "drop a .zip on the window to import".to_string(),
                "up/down: select  e: export to home folder  o: open folder".to_string(),
                String::new(),
            ];
            if app.worlds.is_empty() {
                lines.push("no worlds yet".to_string());
            }
            for (i, w) in app.worlds.iter().enumerate() {
                let marker = if i == app.list_index { ">" } else { " " };
                lines.push(format!("{marker} {w}"));
            }
            lines.join("\n")
        }
        Tab::Launch => {
            let c1 = if app.hide_after_launch { "[x]" } else { "[ ]" };
            let c2 = if app.show_logs_separate { "[x]" } else { "[ ]" };
            let mut lines = vec![
                "up/down: select, enter: launch, d: delete".to_string(),
                format!("1:{c1} hide launcher after launch"),
                format!("2:{c2} show logs in separate terminal"),
                String::new(),
            ];
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
    let status_style = if matches!(app.input_mode, InputMode::ConfirmDelete) {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(app.status.as_str()).style(status_style), chunks[2]);
}

fn draw_browse_splash(f: &mut Frame, elapsed: u64) {
    let left = 3u64.saturating_sub(elapsed).max(1);
    let mut lines: Vec<Line> = crate::art::ART.lines().map(|l| Line::from(l.to_string())).collect();
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Opening browser in {left}...")));
    lines.push(Line::from("press any key to cancel"));
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Browse")),
        f.size(),
    );
}