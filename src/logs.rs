use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::paths;
use crate::stats::fmt_bytes;

pub struct Report {
    pub version: String,
    pub code: Option<i32>,
    pub findings: Vec<(String, String)>,
    pub files: Vec<(String, String)>,
    pub tail: Vec<String>,
    pub tail_name: String,
    pub scroll: usize,
    pub crash_file: Option<String>,
}

static PENDING: Mutex<Option<Report>> = Mutex::new(None);

pub fn take_pending() -> Option<Report> {
    PENDING.lock().unwrap().take()
}

const PATTERNS: [(&[&str], &str); 10] = [
    (&["UnsupportedClassVersionError"], "Wrong Java version. Install the needed one: More > Java"),
    (&["OutOfMemoryError"], "Out of memory. Close other apps or use fewer mods"),
    (&["Incompatible mod set", "Mod resolution failed", "requires version"], "Missing or wrong mod dependency (see the line below)"),
    (&["Mixin apply failed", "MixinApplyError", "InvalidMixinException"], "A mod conflicts with another mod (mod name is in the line)"),
    (&["Duplicate mod", "duplicate mod"], "The same mod is installed twice. Remove one copy"),
    (&["NoSuchMethodError", "NoSuchFieldError"], "A mod is built for another game version"),
    (&["NoClassDefFoundError", "ClassNotFoundException"], "Missing library or API mod (e.g. Fabric API) or wrong version"),
    (&["UnsatisfiedLinkError", "Failed to locate library"], "Natives (.so) not found. Launcher problem, not a mod"),
    (&["Could not find or load main class"], "Broken classpath. Check resolve_debug.log for MISSING"),
    (&["GLFW error", "Pixel format not accelerated", "OpenGL"], "Graphics driver or OpenGL problem"),
];

fn read_lines(p: &Path, last: usize) -> Vec<String> {
    let Ok(bytes) = fs::read(p) else { return Vec::new() };
    let text = String::from_utf8_lossy(&bytes).replace('\t', "    ");
    let all: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let skip = all.len().saturating_sub(last);
    all[skip..].to_vec()
}

fn newest(dir: &Path, prefix: &str) -> Option<(PathBuf, SystemTime)> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .max_by_key(|(_, t)| *t)
}

pub fn on_exit(id: &str, code: Option<i32>, started: SystemTime) {
    let game = paths::game_dir();
    let crash_any = newest(&game.join("crash-reports"), "crash-");
    let hs_any = newest(&game, "hs_err_pid");
    let fresh = |x: &Option<(PathBuf, SystemTime)>| {
        x.as_ref().filter(|(_, t)| *t >= started).map(|(p, _)| p.clone())
    };
    let crash = fresh(&crash_any);
    let hs = fresh(&hs_any);
    let bad_exit = matches!(code, Some(c) if c != 0);
    if !bad_exit && crash.is_none() && hs.is_none() {
        return;
    }

    let latest = read_lines(&game.join("logs/latest.log"), 400);
    let mut all = latest.clone();
    if let Some(p) = &crash {
        all.extend(read_lines(p, 300));
    }
    if let Some(p) = &hs {
        all.extend(read_lines(p, 80));
    }

    let mut findings: Vec<(String, String)> = Vec::new();
    for (needles, msg) in PATTERNS.iter() {
        if let Some(line) = all.iter().find(|l| needles.iter().any(|n| l.contains(n))) {
            let detail: String = line.trim().chars().take(140).collect();
            findings.push((msg.to_string(), detail));
        }
        if findings.len() >= 4 {
            break;
        }
    }
    if findings.len() < 4 {
        if let Some(line) = all.iter().rev().find(|l| l.trim_start().starts_with("Caused by:")) {
            let detail: String = line.trim().chars().take(140).collect();
            findings.push(("Root cause found in the log".to_string(), detail));
        }
    }
    if findings.is_empty() {
        findings.push((
            "No known pattern".to_string(),
            "Open the crash report and read the 'Description' and 'Caused by' lines".to_string(),
        ));
    }

    let rel = |p: &Path| p.strip_prefix(&game).unwrap_or(p).to_string_lossy().to_string();
    let mut candidates: Vec<(String, &str)> = vec![
        ("logs/latest.log".into(), "Main log of the last run: mod loading, errors, warnings"),
        ("logs/debug.log".into(), "Verbose log: Mixin, resources, extra detail"),
    ];
    if let Some((p, _)) = &crash_any {
        candidates.push((rel(p), "Crash report: read 'Description' and 'Caused by'"));
    }
    if let Some((p, _)) = &hs_any {
        candidates.push((rel(p), "Java itself crashed: memory, drivers or natives"));
    }
    candidates.push(("resolve_debug.log".into(), "Launcher: which libraries were found or MISSING"));
    candidates.push(("launch_args.txt".into(), "Launcher: exact Java arguments and classpath"));

    let files: Vec<(String, String)> = candidates
        .into_iter()
        .filter_map(|(name, desc)| {
            let size = fs::metadata(game.join(&name)).ok()?.len();
            Some((format!("{name} ({})", fmt_bytes(size)), desc.to_string()))
        })
        .collect();

    let (tail, tail_name) = if !latest.is_empty() {
        (latest, "logs/latest.log".to_string())
    } else if let Some(p) = crash.as_ref().or(hs.as_ref()) {
        (read_lines(p, 300), rel(p))
    } else {
        (vec!["(no log files found)".to_string()], "-".to_string())
    };
    let scroll = tail.len().saturating_sub(25);

    let report = Report {
        version: id.to_string(),
        code,
        findings,
        files,
        tail,
        tail_name,
        scroll,
        crash_file: crash.or(hs).map(|p| p.to_string_lossy().to_string()),
    };
    *PENDING.lock().unwrap() = Some(report);
}