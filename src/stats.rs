use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{mods, packs, paths, worlds};

#[derive(Clone, Default)]
pub struct Snapshot {
    pub total_secs: u64,
    pub launches: u64,
    pub last_played: u64,
    pub worlds: usize,
    pub worlds_bytes: u64,
    pub biggest_world: Option<(String, u64)>,
    pub mods: usize,
    pub mods_bytes: u64,
    pub packs: usize,
    pub packs_bytes: u64,
    pub data_bytes: u64,
}

static CACHE: Mutex<Option<Snapshot>> = Mutex::new(None);
static SESSION_START: AtomicU64 = AtomicU64::new(0);

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// формат файла: "<всего секунд> <запусков> <последняя игра, unix>"
fn read_playtime() -> (u64, u64, u64) {
    let t = fs::read_to_string(paths::stats_file()).unwrap_or_default();
    let mut it = t.split_whitespace().map(|x| x.parse::<u64>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn dir_size(path: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(path) else { return 0 };
    rd.flatten()
        .map(|e| {
            let Ok(ft) = e.file_type() else { return 0 };
            if ft.is_dir() {
                dir_size(&e.path())
            } else if ft.is_file() {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
                0 // симлинки пропускаем
            }
        })
        .sum()
}

fn refresh() {
    let (total_secs, launches, last_played) = read_playtime();

    let world_list = worlds::list();
    let mut worlds_bytes = 0;
    let mut biggest: Option<(String, u64)> = None;
    for w in &world_list {
        let size = dir_size(&paths::saves_dir().join(w));
        worlds_bytes += size;
        if biggest.as_ref().map_or(true, |(_, b)| size > *b) {
            biggest = Some((w.clone(), size));
        }
    }

    let snap = Snapshot {
        total_secs,
        launches,
        last_played,
        worlds: world_list.len(),
        worlds_bytes,
        biggest_world: biggest,
        mods: mods::list().len(),
        mods_bytes: dir_size(&paths::mods_dir()),
        packs: packs::list().len(),
        packs_bytes: dir_size(&paths::packs_dir()),
        data_bytes: dir_size(&paths::data_dir()),
    };
    *CACHE.lock().unwrap() = Some(snap);
}

/// Запускает фоновое обновление статистики.
pub fn init() {
    std::thread::spawn(|| loop {
        refresh();
        std::thread::sleep(Duration::from_secs(15));
    });
}

/// Мгновенно возвращает последний посчитанный срез (плюс текущая сессия).
pub fn snapshot() -> Snapshot {
    let mut s = CACHE.lock().unwrap().clone().unwrap_or_default();
    let start = SESSION_START.load(Ordering::Relaxed);
    if start != 0 {
        s.total_secs += now_secs().saturating_sub(start);
    }
    s
}

pub fn session_start() {
    SESSION_START.store(now_secs(), Ordering::Relaxed);
}

pub fn session_end() {
    let start = SESSION_START.swap(0, Ordering::Relaxed);
    if start == 0 {
        return;
    }
    let now = now_secs();
    let (total, launches, _) = read_playtime();
    let line = format!("{} {} {}", total + now.saturating_sub(start), launches + 1, now);
    let _ = fs::create_dir_all(paths::data_dir());
    let _ = fs::write(paths::stats_file(), line);
    refresh();
}

pub fn fmt_duration(secs: u64) -> String {
    format!("{}h {:02}m {:02}s", secs / 3600, secs % 3600 / 60, secs % 60)
}

pub fn fmt_bytes(b: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{b} B") } else { format!("{v:.1} {}", UNITS[i]) }
}

pub fn fmt_ago(unix: u64) -> String {
    if unix == 0 {
        return "never".to_string();
    }
    let d = now_secs().saturating_sub(unix);
    match d {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86399 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86400),
    }
}