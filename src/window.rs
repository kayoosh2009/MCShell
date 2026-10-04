use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

const NO_WINDOW: u32 = 0x0800_0000; // не мигать чёрным окном консоли

const DEFS: &str = r#"Add-Type -Name W -Namespace N -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int c); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);';"#;

fn ps(script: &str) -> Result<String, String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &format!("{DEFS}{script}")])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| format!("powershell not found: {e}"))?;
    if !out.status.success() {
        return Err("powershell failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Сворачивает активное окно (окно лаунчера) и возвращает его handle.
pub fn hide_current() -> Result<String, String> {
    let id = ps("$h=[N.W]::GetForegroundWindow(); [N.W]::ShowWindow($h,6)|Out-Null; $h.ToInt64()")?;
    id.parse::<i64>().map(|n| n.to_string()).map_err(|_| "bad window id".to_string())
}

pub fn restore(id: Result<String, String>) {
    let Ok(id) = id else { return };
    let Ok(n) = id.parse::<i64>() else { return };
    let _ = ps(&format!(
        "$h=[IntPtr]{n}; [N.W]::ShowWindow($h,9)|Out-Null; [N.W]::SetForegroundWindow($h)|Out-Null"
    ));
}

pub fn open_folder(path: &std::path::Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("explorer")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn open_url(url: &str) {
    // rundll32 корректно открывает ссылки с & и mailto:, в отличие от cmd start
    let _ = Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .creation_flags(NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}