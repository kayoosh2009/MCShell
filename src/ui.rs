use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, InputMode, Tab};

fn kv(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label:<15}"), Style::default().fg(Color::Cyan)),
        Span::raw(value),
    ])
}

fn dim(s: &str) -> Line<'static> {
    Line::styled(format!(" {s}"), Style::default().fg(Color::DarkGray))
}

fn head(s: &str) -> Line<'static> {
    Line::styled(format!(" {s}"), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
}

fn key_lines(keys: &[(&str, &str)]) -> Vec<Line<'static>> {
    keys.iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<10}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(d.to_string(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

fn panel(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(format!(" {title} "), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
}

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
        .block(panel("MCShell"))
        .select(app.current_tab.index())
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD | Modifier::UNDERLINED));
    f.render_widget(tabs, chunks[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    let item = |i: usize, s: String| -> Line<'static> {
        if i == app.list_index {
            Line::styled(format!(" ▶ {s}"), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Line::from(format!("   {s}"))
        }
    };

    let mut left: Vec<Line> = Vec::new();
    let mut stats: Vec<Line> = Vec::new();
    let keys: Vec<(&str, &str)>;

    match app.current_tab {
        Tab::Profile => {
            if let InputMode::EditingUsername = app.input_mode {
                left.push(kv("New username", format!("{}_", app.input_buffer)));
            } else {
                left.push(kv("Username", app.profile.username.clone()));
                left.push(kv("UUID", app.profile.offline_uuid()));
            }
            use crate::stats::{fmt_ago, fmt_bytes, fmt_duration};
            let s = crate::stats::snapshot();
            let biggest = match &s.biggest_world {
                Some((n, b)) => format!("{n} ({})", fmt_bytes(*b)),
                None => "-".to_string(),
            };
            stats.push(head("Playing"));
            stats.push(kv("Playtime", fmt_duration(s.total_secs)));
            stats.push(kv("Launches", s.launches.to_string()));
            stats.push(kv("Last played", fmt_ago(s.last_played)));
            stats.push(Line::from(""));
            stats.push(head("Storage"));
            stats.push(kv("Worlds", format!("{} ({})", s.worlds, fmt_bytes(s.worlds_bytes))));
            stats.push(kv("Biggest world", biggest));
            stats.push(kv("Mods", format!("{} ({})", s.mods, fmt_bytes(s.mods_bytes))));
            stats.push(kv("Texture packs", format!("{} ({})", s.packs, fmt_bytes(s.packs_bytes))));
            stats.push(kv("Launcher data", fmt_bytes(s.data_bytes)));
            keys = vec![("e", "edit username"), ("←/→ Tab", "switch tab"), ("q", "quit")];
        }
        Tab::Versions => {
            if let InputMode::EditingVersion = app.input_mode {
                left.push(kv("Version id", format!("{}_", app.input_buffer)));
            }
            left.push(kv("Installed", app.installed.join(", ")));
            left.push(Line::from(""));
            for (i, v) in app.remote_versions.iter().take(25).enumerate() {
                left.push(item(i, format!("{} ({})", v.id, v.kind)));
            }
            keys = vec![
                ("i", "type version id"),
                ("r", "fetch version list"),
                ("↑/↓", "select"),
                ("Enter", "install selected"),
                ("f", "install Fabric"),
            ];
        }
        Tab::Mods => {
            if app.mods.is_empty() {
                left.push(dim("no mods yet"));
            }
            for (i, m) in app.mods.iter().enumerate() {
                let c = if m.enabled { "[x]" } else { "[ ]" };
                left.push(item(i, format!("{c} {}", m.name)));
            }
            keys = vec![
                ("drop .jar", "add mod"),
                ("↑/↓", "select"),
                ("Space", "enable / disable"),
                ("d", "delete"),
                ("o", "open folder"),
                ("b", "browse online"),
            ];
        }
        Tab::Skins => {
            left.push(kv("Skin", if app.has_skin { "set" } else { "not set" }.to_string()));
            left.push(Line::from(""));
            left.extend(key_lines(&[("drop PNG", "set skin")]));
            keys = vec![];
        }
        Tab::Textures => {
            if app.packs.is_empty() {
                left.push(dim("no texture packs yet"));
            }
            for (i, p) in app.packs.iter().enumerate() {
                left.push(item(i, p.clone()));
            }
            left.push(Line::from(""));
            left.push(dim("enable in game: Options > Resource Packs"));
            keys = vec![
                ("drop .zip", "add pack"),
                ("↑/↓", "select"),
                ("d", "delete"),
                ("o", "open folder"),
                ("b", "browse online"),
            ];
        }
        Tab::Worlds => {
            if app.worlds.is_empty() {
                left.push(dim("no worlds yet"));
            }
            for (i, w) in app.worlds.iter().enumerate() {
                left.push(item(i, w.clone()));
            }
            keys = vec![
                ("drop .zip", "import world"),
                ("↑/↓", "select"),
                ("e", "export to home"),
                ("o", "open folder"),
            ];
        }
        Tab::Launch => {
            let c1 = if app.hide_after_launch { "[x]" } else { "[ ]" };
            let c2 = if app.show_logs_separate { "[x]" } else { "[ ]" };
            left.push(kv("Options", String::new()));
            left.push(Line::from(format!("  {c1} hide launcher after launch")));
            left.push(Line::from(format!("  {c2} logs in separate terminal")));
            left.push(Line::from(""));
            for (i, v) in app.installed.iter().enumerate() {
                left.push(item(i, v.clone()));
            }
            keys = vec![
                ("↑/↓", "select version"),
                ("Enter", "launch"),
                ("d", "delete version"),
                ("1", "toggle hide launcher"),
                ("2", "toggle separate logs"),
            ];
        }
    }

    f.render_widget(
        Paragraph::new(left).wrap(Wrap { trim: false }).block(panel(app.current_tab.title())),
        cols[0],
    );

    match app.current_tab {
        Tab::Profile => {
            let rv = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(keys.len() as u16 + 2)])
                .split(cols[1]);
            f.render_widget(Paragraph::new(stats).block(panel("Stats")), rv[0]);
            f.render_widget(Paragraph::new(key_lines(&keys)).block(panel("Keys")), rv[1]);
        }
        Tab::Skins => {
            if app.has_skin {
                if let Ok(img) = image::open(crate::paths::skin_file()) {
                    let lines = crate::skin_view::skin_to_lines(&img, 32, 32);
                    f.render_widget(Paragraph::new(lines).block(panel("Skin preview")), cols[1]);
                }
            } else {
                f.render_widget(Paragraph::new(vec![dim("no skin yet")]).block(panel("Skin preview")), cols[1]);
            }
        }
        _ => {
            f.render_widget(Paragraph::new(key_lines(&keys)).block(panel("Keys")), cols[1]);
        }
    }

    let status_style = if matches!(app.input_mode, InputMode::ConfirmDelete) {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
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