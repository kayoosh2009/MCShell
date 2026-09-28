use std::process::Command;

fn tool() -> &'static str {
    if std::env::var("WAYLAND_DISPLAY").is_ok() { "kdotool" } else { "xdotool" }
}

pub fn hide_current() -> Result<String, String> {
    let t = tool();
    let output = Command::new(t).arg("getactivewindow").output()
        .map_err(|e| format!("{t} not found: {e}"))?;
    if !output.status.success() {
        return Err(format!("{t} getactivewindow failed"));
    }
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let _ = Command::new(t).arg("windowminimize").arg(&id).output();
    Ok(id)
}

pub fn restore(id: Result<String, String>) {
    if let Ok(id) = id {
        let _ = Command::new(tool()).arg("windowactivate").arg(id).output();
    }
}

pub fn open_folder(path: &std::path::Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("xdg-open")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn open_url(url: &str) {
    let _ = Command::new("xdg-open")
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}