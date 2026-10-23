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
    if let Some(r) = &app.crash {
        draw_crash(f, r);
        return;
    }
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
            left.push(Line::from(""));
            left.push(dim("Press [Enter] / [Space] to open Web Dashboard"));

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

            keys = vec![
                ("Enter/Space", "open Web Dashboard"),
                ("e", "edit username"),
                ("←/→ Tab", "switch tab"),
                ("q", "quit"),
            ];
        }
        Tab::Launch => {
            if app.installed.is_empty() {
                left.push(dim("no versions installed yet"));
                left.push(dim("use Web Dashboard to download versions"));
            } else {
                for (i, v) in app.installed.iter().enumerate() {
                    left.push(item(i, v.clone()));
                }
            }
            keys = vec![
                ("↑/↓", "select version"),
                ("Enter", "launch game"),
                ("d", "delete version"),
            ];
        }
        Tab::Faq => {
            use crate::settings::{DISCORD, EMAIL, GITHUB, TELEGRAM};
            left.push(Line::styled(" FAQ & Information", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)));
            left.push(Line::from(""));
            left.push(dim("• Fast & lightweight Minecraft launcher with TUI."));
            left.push(dim("• All mods, skins, and advanced settings are configured via Web UI."));
            left.push(dim("• Press Enter on Profile tab to launch local Web panel."));
            left.push(Line::from(""));
            left.push(Line::styled(" Links / Contacts", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)));
            left.push(Line::styled(format!("   [g] GitHub:   {GITHUB}"), Style::default().fg(Color::DarkGray)));
            left.push(Line::styled(format!("   [d] Discord:  {DISCORD}"), Style::default().fg(Color::DarkGray)));
            left.push(Line::styled(format!("   [t] Telegram: {TELEGRAM}"), Style::default().fg(Color::DarkGray)));
            left.push(Line::styled(format!("   [m] Email:    {EMAIL}"), Style::default().fg(Color::DarkGray)));

            keys = vec![
                ("g", "open GitHub"),
                ("d", "open Discord"),
                ("t", "open Telegram"),
                ("m", "write email"),
            ];
        }
    } for Minecraft."));
                    left.push(dim("Fabric, mods, texture packs, worlds and skins"));
                    left.push(dim("in one place, with an online Modrinth browser."));
                    left.push(Line::from(""));
                    left.push(head("Contacts"));
                    left.push(kv("GitHub", GITHUB.to_string()));
                    left.push(kv("Email", EMAIL.to_string()));
                    left.push(kv("Discord", DISCORD.to_string()));
                    left.push(kv("Telegram", TELEGRAM.to_string()));
                    keys = vec![("g", "open GitHub"), ("m", "write email"), ("d", "open Discord"), ("t", "open Telegram"), ("1 2 3", "switch page")];
                }
            }
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

fn draw_crash(f: &mut Frame, r: &crate::logs::Report) {
    let red = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let mut diag: Vec<Line> = vec![
        kv("Version", r.version.clone()),
        kv("Exit code", r.code.map_or("-".to_string(), |c| c.to_string())),
    ];
    for (msg, detail) in &r.findings {
        diag.push(Line::styled(format!(" ▸ {msg}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        diag.push(Line::styled(format!("   {detail}"), Style::default().fg(Color::DarkGray)));
    }

    let files: Vec<Line> = r
        .files
        .iter()
        .map(|(name, desc)| {
            Line::from(vec![
                Span::styled(format!(" {name}"), Style::default().fg(Color::Cyan)),
                Span::styled(format!("  {desc}"), Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(diag.len() as u16 + 2),
            Constraint::Length(files.len() as u16 + 2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(f.size());

    f.render_widget(
        Paragraph::new(diag).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(Span::styled(" Game crashed ", red)),
        ),
        chunks[0],
    );
    f.render_widget(Paragraph::new(files).block(panel("Log files (in game folder)")), chunks[1]);

    let tail: Vec<Line> = r.tail.iter().skip(r.scroll).map(|l| Line::from(l.clone())).collect();
    f.render_widget(Paragraph::new(tail).block(panel(&r.tail_name)), chunks[2]);

    f.render_widget(
        Paragraph::new("↑/↓ PgUp/PgDn: scroll   o: open logs folder   c: open crash report   Esc/Enter: close")
            .style(Style::default().fg(Color::Gray)),
        chunks[3],
    );
}