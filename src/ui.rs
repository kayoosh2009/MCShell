use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

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

fn load_bg_image() -> slint::Image {
    let candidates = ["backgroud.png", "assets/backgroud.png", "background.png", "assets/background.png"];
    for path in candidates {
        if let Ok(img) = image::open(path) {
            let small = img.thumbnail(960, 620).to_rgba8();
            let blurred = image::imageops::blur(&small, 5.0);
            let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                blurred.as_raw(),
                blurred.width(),
                blurred.height(),
            );
            return slint::Image::from_rgba8(buf);
        }
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
    win.set_bg_image(load_bg_image());
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