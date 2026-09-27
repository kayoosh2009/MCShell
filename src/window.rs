use std::process::Command;

fn tool() -> &'static str {
    if std::env::var("WAYLAND_DISPLAY").is_ok() { "kdotool" } else { "xdotool" }
}

pub fn hide_current() -> Option<String> {
    let t = tool();
    let id = Command::new(t).arg("getactivewindow").output().ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    let _ = Command::new(t).arg("windowminimize").arg(&id).output();
    Some(id)
}

pub fn restore(id: Option<String>) {
    if let Some(id) = id {
        let _ = Command::new(tool()).arg("windowactivate").arg(id).output();
    }
}