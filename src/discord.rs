use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::json;

// Вставь сюда Application ID из Discord Developer Portal
const APP_ID: &str = "1554822011484643498";

static CONN: Mutex<Option<UnixStream>> = Mutex::new(None);
static START: AtomicU64 = AtomicU64::new(0);

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn connect() -> Option<UnixStream> {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());
    
    // обычный Discord, flatpak и snap
    let dirs = [
        base.clone(),
        format!("{base}/app/com.discordapp.Discord"),
        format!("{base}/snap.discord"),
    ];
    
    for dir in dirs {
        for i in 0..10 {
            if let Ok(s) = UnixStream::connect(format!("{dir}/discord-ipc-{i}")) {
                return Some(s);
            }
        }
    }
    None
}

fn log(msg: &str) {
    let dir = crate::paths::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("discord.log"))
    {
        let _ = writeln!(f, "{msg}");
    }
}

fn send(s: &mut UnixStream, op: u32, payload: &str) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload.as_bytes());
    s.write_all(&buf)
}

fn recv(s: &mut UnixStream) -> std::io::Result<String> {
    let mut head = [0u8; 8];
    s.read_exact(&mut head)?;
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if len > 64 * 1024 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too big"));
    }
    let mut body = vec![0u8; len];
    s.read_exact(&mut body)?;
    Ok(String::from_utf8_lossy(&body).to_string())
}

/// Подключается к Discord. Если Discord не запущен, тихо ничего не делает.
pub fn init() {
    if APP_ID.starts_with("PUT_") {
        log("APP_ID not set");
        return;
    }
    let Some(mut s) = connect() else {
        log("no discord-ipc socket found (is Discord running? flatpak/snap?)");
        return;
    };
    log("socket connected");
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(2)));
    
    let hello = json!({ "v": 1, "client_id": APP_ID }).to_string();
    if let Err(e) = send(&mut s, 0, &hello) {
        log(&format!("handshake send failed: {e}"));
        return;
    }
    match recv(&mut s) {
        Ok(body) => {
            log(&format!("handshake reply: {body}"));
            if !body.contains("READY") {
                return; // Discord вернул ошибку (например, неверный APP_ID)
            }
        }
        Err(e) => {
            log(&format!("handshake recv failed: {e}"));
            return;
        }
    }
    START.store(now_secs(), Ordering::Relaxed);
    *CONN.lock().unwrap() = Some(s);
    set("In launcher", "Choosing a version");
}

/// Обновляет статус. Можно вызывать из любого потока.
pub fn set(details: &str, state: &str) {
    let mut guard = CONN.lock().unwrap();
    let Some(s) = guard.as_mut() else { return };
    
    // 👇 ЗДЕСЬ ДОБАВЛЕН МАССИВ "buttons" 👇
    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": std::process::id(),
            "activity": {
                "details": details,
                "state": state,
                "timestamps": { "start": START.load(Ordering::Relaxed) },
                "assets": { 
                    "large_image": "mcshell", 
                    "large_text": "MCShell" 
                },
                "buttons": [
                    {
                        "label": "Download MCShell",
                        "url": "https://kayoosh2009.github.io/MCShell/"
                    },
                    {
                        "label": "MCShell Discord",
                        "url": "https://discord.gg/Zy4jW3m2z8"
                    }
                ]
            }
        },
        "nonce": now_secs().to_string()
    })
    .to_string();

    match send(s, 1, &payload).and_then(|_| recv(s)) {
        Ok(body) => log(&format!("set_activity reply: {body}")),
        Err(e) => {
            log(&format!("set_activity failed: {e}"));
            *guard = None;
        }
    }
}

/// Закрывает соединение, Discord сам уберёт статус.
pub fn shutdown() {
    *CONN.lock().unwrap() = None;
}