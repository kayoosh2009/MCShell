use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode, VecModel};

use crate::app::{App, Tab};
use crate::{paths, window};

slint::include_modules!();

fn kv_item(label: &str, value: String) -> KeyValueItem {
    KeyValueItem {
        label: SharedString::from(label),
        value: SharedString::from(value),
    }
}

fn load_skin_image() -> slint::Image {
    if let Ok(img) = image::open(paths::skin_file()) {
        return crate::skin_view::render_preview(&img);
    }
    slint::Image::default()
}

fn sync_ui(win: &MainWindow, app: &App) {
    win.set_status(SharedString::from(app.status.as_str()));
    win.set_username(SharedString::from(app.profile.username.as_str()));
    win.set_uuid(SharedString::from(app.profile.offline_uuid()));
    win.set_hide_after_launch(app.hide_after_launch);
    win.set_show_logs_separate(app.show_logs_separate);
    win.set_discord_on(app.discord_on);
    win.set_has_skin(app.has_skin);

    if app.has_skin {
        win.set_skin_preview(load_skin_image());
    }

    let progress: Vec<SharedString> = app.progress_log.iter().map(SharedString::from).collect();
    win.set_progress_log(ModelRc::from(Rc::new(VecModel::from(progress))));

    let installed: Vec<SharedString> = app.installed.iter().map(SharedString::from).collect();
    win.set_installed_versions(ModelRc::from(Rc::new(VecModel::from(installed))));

    let remote: Vec<VersionItem> = app
        .remote_versions
        .iter()
        .take(50)
        .map(|v| VersionItem {
            id: SharedString::from(v.id.as_str()),
            kind: SharedString::from(v.kind.as_str()),
        })
        .collect();
    win.set_remote_versions(ModelRc::from(Rc::new(VecModel::from(remote))));

    let mods_list: Vec<ModItem> = app
        .mods
        .iter()
        .map(|m| ModItem {
            name: SharedString::from(m.name.as_str()),
            enabled: m.enabled,
        })
        .collect();
    win.set_mods(ModelRc::from(Rc::new(VecModel::from(mods_list))));

    let packs: Vec<SharedString> = app.packs.iter().map(SharedString::from).collect();
    win.set_packs(ModelRc::from(Rc::new(VecModel::from(packs))));

    let worlds: Vec<SharedString> = app.worlds.iter().map(SharedString::from).collect();
    win.set_worlds(ModelRc::from(Rc::new(VecModel::from(worlds))));

    let javas: Vec<JavaItem> = crate::java::JAVAS
        .iter()
        .map(|(m, note)| JavaItem {
            major: *m as i32,
            installed: crate::java::is_installed(*m),
            note: SharedString::from(*note),
        })
        .collect();
    win.set_java_list(ModelRc::from(Rc::new(VecModel::from(javas))));

    use crate::stats::{fmt_ago, fmt_bytes, fmt_duration};
    let s = crate::stats::snapshot();
    let biggest = match &s.biggest_world {
        Some((n, b)) => format!("{n} ({})", fmt_bytes(*b)),
        None => "-".to_string(),
    };
    win.set_playing_stats(ModelRc::from(Rc::new(VecModel::from(vec![
        kv_item("Playtime", fmt_duration(s.total_secs)),
        kv_item("Launches", s.launches.to_string()),
        kv_item("Last played", fmt_ago(s.last_played)),
    ]))));
    win.set_storage_stats(ModelRc::from(Rc::new(VecModel::from(vec![
        kv_item("Worlds", format!("{} ({})", s.worlds, fmt_bytes(s.worlds_bytes))),
        kv_item("Biggest world", biggest),
        kv_item("Mods", format!("{} ({})", s.mods, fmt_bytes(s.mods_bytes))),
        kv_item("Texture packs", format!("{} ({})", s.packs, fmt_bytes(s.packs_bytes))),
        kv_item("Launcher data", fmt_bytes(s.data_bytes)),
    ]))));

    use crate::settings::{DISCORD, EMAIL, GITHUB, TELEGRAM};
    win.set_about_github(SharedString::from(GITHUB));
    win.set_about_email(SharedString::from(EMAIL));
    win.set_about_discord(SharedString::from(DISCORD));
    win.set_about_telegram(SharedString::from(TELEGRAM));

    let browse_secs = app
        .browse_launch
        .as_ref()
        .map(|(start, _)| 3i32.saturating_sub(start.elapsed().as_secs() as i32).max(1))
        .unwrap_or(0);
    win.set_browse_seconds(browse_secs);

    if let Some(r) = &app.crash {
        win.set_crash_visible(true);
        win.set_crash_version(SharedString::from(r.version.as_str()));
        win.set_crash_code(SharedString::from(r.code.map_or("-".to_string(), |c| c.to_string())));
        let findings: Vec<CrashFinding> = r
            .findings
            .iter()
            .map(|(msg, detail)| CrashFinding {
                msg: SharedString::from(msg.as_str()),
                detail: SharedString::from(detail.as_str()),
            })
            .collect();
        win.set_crash_findings(ModelRc::from(Rc::new(VecModel::from(findings))));
        win.set_crash_tail_name(SharedString::from(r.tail_name.as_str()));
        win.set_crash_tail_text(SharedString::from(r.tail.join("\n")));
    } else {
        win.set_crash_visible(false);
    }
}

pub fn run() -> Result<()> {
    let win = MainWindow::new()?;
    let app = Rc::new(RefCell::new(App::new()));
    sync_ui(&win, &app.borrow());

    let bind = |cb: Box<dyn Fn(&mut App)>| {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        move || {
            let mut a = app.borrow_mut();
            cb(&mut a);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        }
    };

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_tab_changed(move |idx| {
            let mut a = app.borrow_mut();
            if let Some(tab) = Tab::ALL.get(idx as usize).copied() {
                a.current_tab = tab;
                a.on_tab_change();
            }
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_save_username(move |name| {
            let mut a = app.borrow_mut();
            let clean = name.trim().to_string();
            if !clean.is_empty() {
                a.profile.set_username(clean);
                a.status = "username saved".to_string();
            }
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    win.on_fetch_versions(bind(Box::new(|a| a.fetch_remote_versions())));

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_install_version(move |id| {
            let mut a = app.borrow_mut();
            let clean = id.trim().to_string();
            if !clean.is_empty() {
                a.last_vanilla = Some(clean.clone());
                a.spawn_version_install(clean);
            }
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_install_fabric(move |id| {
            let mut a = app.borrow_mut();
            let clean = id.trim().to_string();
            let target = if !clean.is_empty() { Some(clean) } else { a.last_vanilla.clone() };
            match target {
                Some(mc) => a.spawn_fabric_install(mc),
                None => a.status = "type or install a vanilla version first".to_string(),
            }
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_delete_version(move |id| {
            let mut a = app.borrow_mut();
            a.delete_version(&id);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_launch_version(move |id| {
            let mut a = app.borrow_mut();
            a.launch_version(id.to_string());
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_toggle_mod(move |idx| {
            let mut a = app.borrow_mut();
            a.toggle_mod(idx as usize);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_delete_mod(move |idx| {
            let mut a = app.borrow_mut();
            a.delete_mod(idx as usize);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    win.on_open_mods_folder(|| window::open_folder(&paths::mods_dir()));
    win.on_browse_online(bind(Box::new(|a| a.start_browse())));

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_delete_pack(move |idx| {
            let mut a = app.borrow_mut();
            a.delete_pack(idx as usize);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    win.on_open_packs_folder(|| window::open_folder(&paths::packs_dir()));

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_export_world(move |idx| {
            let mut a = app.borrow_mut();
            a.export_world(idx as usize);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    win.on_open_worlds_folder(|| window::open_folder(&paths::saves_dir()));

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_toggle_setting(move |i| {
            let mut a = app.borrow_mut();
            a.toggle_setting(i as usize);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_install_java(move |major| {
            let mut a = app.borrow_mut();
            a.spawn_java_install(major as u32);
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        win.on_remove_java(move |major| {
            let mut a = app.borrow_mut();
            a.status = match crate::java::remove(major as u32) {
                Ok(()) => format!("java {major} removed"),
                Err(e) => format!("remove error: {e}"),
            };
            if let Some(w) = weak.upgrade() {
                sync_ui(&w, &a);
            }
        });
    }

    win.on_open_link(|url| window::open_url(&url));
    win.on_cancel_browse(bind(Box::new(|a| a.cancel_browse())));
    win.on_close_crash(bind(Box::new(|a| a.crash = None)));
    win.on_open_logs_folder(|| window::open_folder(&paths::game_dir().join("logs")));

    {
        let app = Rc::clone(&app);
        win.on_open_crash_file(move || {
            if let Some(r) = &app.borrow().crash {
                if let Some(p) = &r.crash_file {
                    window::open_url(p);
                }
            }
        });
    }

    let timer = Timer::default();
    {
        let app = Rc::clone(&app);
        let weak = win.as_weak();
        timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
            let mut a = app.borrow_mut();
            let mut dirty = a.poll_progress();
            dirty |= a.poll_browse();
            if let Some(r) = crate::logs::take_pending() {
                a.crash = Some(r);
                dirty = true;
            }
            if dirty {
                if let Some(w) = weak.upgrade() {
                    sync_ui(&w, &a);
                }
            }
        });
    }

    win.run()?;
    Ok(())
}

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
                Tab::More => {
            let pages = ["1 Settings", "2 Java", "3 About"];
            let mut hdr: Vec<Span> = Vec::new();
            for (i, p) in pages.iter().enumerate() {
                let st = if i == app.more_page {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                hdr.push(Span::styled(format!(" {p} "), st));
            }
            left.push(Line::from(hdr));
            left.push(Line::from(""));

            match app.more_page {
                0 => {
                    let rows = [
                        ("Discord presence", app.discord_on),
                        ("Hide launcher after launch", app.hide_after_launch),
                        ("Logs in separate terminal", app.show_logs_separate),
                    ];
                    for (i, (name, on)) in rows.iter().enumerate() {
                        let c = if *on { "[x]" } else { "[ ]" };
                        left.push(item(i, format!("{c} {name}")));
                    }
                    keys = vec![("↑/↓", "select"), ("Enter", "toggle yes / no"), ("1 2 3", "switch page")];
                }
                1 => {
                    left.push(dim("Launcher picks the right Java automatically."));
                    left.push(dim("If it is not installed, system java is used."));
                    left.push(Line::from(""));
                    for (i, (m, note)) in crate::java::JAVAS.iter().enumerate() {
                        let st = if crate::java::is_installed(*m) { "installed" } else { "not installed" };
                        left.push(item(i, format!("Java {m:<3} {st:<14} {note}")));
                    }
                    keys = vec![("↑/↓", "select"), ("Enter", "download"), ("d", "delete"), ("1 2 3", "switch page")];
                }
                _ => {
                    use crate::settings::{DISCORD, EMAIL, GITHUB, TELEGRAM};
                    left.push(head("MCShell"));
                    left.push(dim("A fast terminal launcher for Minecraft."));
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