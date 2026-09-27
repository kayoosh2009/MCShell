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