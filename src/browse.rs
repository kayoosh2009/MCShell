use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::{paths, versions};

const API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "MCShell/0.1.0 (github.com/kayoosh2009/MCShell)";
const INDEX_HTML: &str = include_str!("web/index.html");
const STYLE_CSS: &str = include_str!("web/style.css");
const SCRIPT_JS: &str = include_str!("web/script.js");

struct State {
    token: String,
    tx: Mutex<Sender<String>>,
}

pub struct BrowseServer {
    base: String,
    pub events: Receiver<String>,
}

impl BrowseServer {
    pub fn url(&self, kind: &str) -> String {
        format!("{}&kind={kind}", self.base)
    }
}

pub struct Context {
    pub mc_version: Option<String>,
    pub loader: Option<String>,
}

pub fn start() -> Result<BrowseServer> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let token = random_token()?;
    let (tx, rx) = mpsc::channel();
    let state = Arc::new(State { token: token.clone(), tx: Mutex::new(tx) });

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let state = state.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &state);
            });
        }
    });

    Ok(BrowseServer { base: format!("http://127.0.0.1:{port}/?t={token}"), events: rx })
}

fn random_token() -> Result<String> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).map_err(|e| anyhow!("random error: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn parse_query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn respond(stream: &mut TcpStream, code: u16, ctype: &str, body: &[u8]) -> Result<()> {
    let reason = match code {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn json_response(stream: &mut TcpStream, code: u16, v: &Value) -> Result<()> {
    respond(stream, code, "application/json", &serde_json::to_vec(v)?)
}

fn safe(s: &str) -> Option<String> {
    let ok = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    ok.then(|| s.to_string())
}

fn kind_of(q: &HashMap<String, String>) -> &'static str {
    if q.get("kind").map(String::as_str) == Some("resourcepack") {
        "resourcepack"
    } else {
        "mod"
    }
}

pub fn detect_context() -> Context {
    let installed = versions::installed_versions();
    let fabric_mc = installed.iter().find_map(|id| {
        let rest = id.strip_prefix("fabric-loader-")?;
        let (_, mc) = rest.split_once('-')?;
        Some(mc.to_string())
    });
    if let Some(mc) = fabric_mc {
        return Context { mc_version: safe(&mc), loader: Some("fabric".to_string()) };
    }
    let vanilla = installed.into_iter().find(|id| !id.starts_with("fabric-loader-"));
    Context { mc_version: vanilla.and_then(|v| safe(&v)), loader: None }
}

fn handle(mut stream: TcpStream, state: &Arc<State>) -> Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut parts = req.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let q = parse_query(query);

    // Статику отдаём без токена: <link>/<script src> идут по относительным
    // путям без query, а сами файлы ничего не меняют и не читают.
    match (method, path) {
        ("GET", "/") if q.get("t").map(String::as_str) == Some(state.token.as_str()) => {
            return respond(&mut stream, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes());
        }
        ("GET", "/") => return respond(&mut stream, 403, "text/plain", b"forbidden"),
        ("GET", "/style.css") => return respond(&mut stream, 200, "text/css", STYLE_CSS.as_bytes()),
        ("GET", "/script.js") => return respond(&mut stream, 200, "text/javascript", SCRIPT_JS.as_bytes()),
        _ => {}
    }

    if q.get("t").map(String::as_str) != Some(state.token.as_str()) {
        return respond(&mut stream, 403, "text/plain", b"forbidden");
    }

    match (method, path) {
        ("GET", "/api/info") => {
            let ctx = detect_context();
            json_response(&mut stream, 200, &json!({ "version": ctx.mc_version, "loader": ctx.loader }))
        }
        ("GET", "/api/targets") => json_response(&mut stream, 200, &json!(list_targets())),
        ("GET", "/api/installed") => {
            json_response(&mut stream, 200, &json!(installed_ids(kind_of(&q))))
        }
        ("GET", "/api/tags") => match fetch_tags(kind_of(&q)) {
            Ok(v) => json_response(&mut stream, 200, &v),
            Err(e) => json_response(&mut stream, 500, &json!({ "error": e.to_string() })),
        },
        ("GET", "/api/project") => {
            let id = q.get("id").cloned().unwrap_or_default();
            match fetch_project(&id) {
                Ok(v) => json_response(&mut stream, 200, &v),
                Err(e) => json_response(&mut stream, 500, &json!({ "error": e.to_string() })),
            }
        }
        ("GET", "/api/search") => match search(&q) {
            Ok(v) => json_response(&mut stream, 200, &v),
            Err(e) => json_response(&mut stream, 500, &json!({ "error": e.to_string() })),
        },
        ("POST", "/api/install") => {
            let id = q.get("id").cloned().unwrap_or_default();
            let ctx = Context {
                mc_version: q.get("mc").and_then(|m| safe(m)),
                loader: q.get("loader").filter(|l| !l.is_empty()).and_then(|l| safe(l)),
            };
            let result = install_root(&id, kind_of(&q), &ctx);
            match result {
                Ok(done) => {
                    let _ = state.tx.lock().unwrap().send(format!("installed {}", done.join(", ")));
                    json_response(&mut stream, 200, &json!({ "ok": true, "files": done }))
                }
                Err(e) => json_response(&mut stream, 200, &json!({ "ok": false, "error": e.to_string() })),
            }
        }
        _ => respond(&mut stream, 404, "text/plain", b"not found"),
    }
}

fn search(q: &HashMap<String, String>) -> Result<Value> {
    let kind = kind_of(q);
    let ctx = detect_context();

    let mut facets = vec![format!("[\"project_type:{kind}\"]")];
    if let Some(v) = &ctx.mc_version {
        facets.push(format!("[\"versions:{v}\"]"));
    }
    if kind == "mod" {
        if let Some(l) = &ctx.loader {
            facets.push(format!("[\"categories:{l}\"]"));
        }
    }
    let tags: Vec<String> = q.get("tags").map(|s| s.split(',').filter_map(safe).collect()).unwrap_or_default();
    if !tags.is_empty() {
        let group: Vec<String> = tags.iter().map(|t| format!("\"categories:{t}\"")).collect();
        facets.push(format!("[{}]", group.join(",")));
    }
    let facets = format!("[{}]", facets.join(","));

    let query = q.get("q").map(String::as_str).unwrap_or("");
    let offset = q.get("offset").and_then(|o| o.parse::<u32>().ok()).unwrap_or(0).to_string();

    let body: Value = ureq::get(&format!("{API}/search"))
        .set("User-Agent", USER_AGENT)
        .query("query", query)
        .query("facets", &facets)
        .query("limit", "20")
        .query("offset", &offset)
        .call()?
        .into_json()?;
    Ok(body)
}

fn list_targets() -> Vec<Value> {
    versions::installed_versions()
        .into_iter()
        .map(|id| {
            if let Some(rest) = id.strip_prefix("fabric-loader-") {
                if let Some((_, mc)) = rest.split_once('-') {
                    return json!({ "id": id, "mc": mc, "loader": "fabric", "label": format!("{mc} (fabric)") });
                }
            }
            json!({ "id": id, "mc": id, "loader": Value::Null, "label": format!("{id} (vanilla)") })
        })
        .collect()
}

fn fetch_tags(kind: &str) -> Result<Value> {
    let list: Value = ureq::get(&format!("{API}/tag/category")).set("User-Agent", USER_AGENT).call()?.into_json()?;
    let filtered: Vec<Value> = list
        .as_array()
        .ok_or_else(|| anyhow!("bad tag response"))?
        .iter()
        .filter(|c| c["project_type"] == kind)
        .map(|c| json!({ "name": c["name"] }))
        .collect();
    Ok(json!(filtered))
}

fn fetch_project(id: &str) -> Result<Value> {
    if safe(id).is_none() {
        bail!("bad project id");
    }
    let p: Value = ureq::get(&format!("{API}/project/{id}")).set("User-Agent", USER_AGENT).call()?.into_json()?;
    let gallery: Vec<Value> = p["gallery"]
        .as_array()
        .map(|a| a.iter().map(|g| json!({ "url": g["url"], "title": g["title"] })).collect())
        .unwrap_or_default();
    Ok(json!({ "title": p["title"], "body": p["body"], "gallery": gallery }))
}

fn install_root(id: &str, kind: &str, ctx: &Context) -> Result<Vec<String>> {
    let mut done = Vec::new();
    install(id, kind, ctx, &mut Vec::new(), &mut done, 0)?;
    Ok(done)
}

fn record_installed(kind: &str, id: &str, filename: &str) {
    let _ = std::fs::create_dir_all(paths::data_dir());
    let line = format!("{kind}|{id}|{filename}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths::installed_file())
    {
        let _ = f.write_all(line.as_bytes());
    }
}

fn installed_ids(kind: &str) -> Vec<String> {
    let dir = if kind == "mod" { paths::mods_dir() } else { paths::packs_dir() };
    let text = std::fs::read_to_string(paths::installed_file()).unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let mut p = line.splitn(3, '|');
        let (Some(k), Some(id), Some(file)) = (p.next(), p.next(), p.next()) else { continue };
        if k != kind {
            continue;
        }
        // файл должен реально лежать в папке (включая выключенные моды)
        let exists = dir.join(file).is_file() || dir.join(format!("{file}.disabled")).is_file();
        if exists && !out.iter().any(|x| x == id) {
            out.push(id.to_string());
        }
    }
    out
}

fn install(id: &str, kind: &str, ctx: &Context, seen: &mut Vec<String>, done: &mut Vec<String>, depth: u8) -> Result<()> {
    if depth > 3 || seen.iter().any(|s| s == id) {
        return Ok(());
    }
    if safe(id).is_none() {
        bail!("bad project id");
    }
    seen.push(id.to_string());

    let mut req = ureq::get(&format!("{API}/project/{id}/version")).set("User-Agent", USER_AGENT);
    if let Some(v) = &ctx.mc_version {
        req = req.query("game_versions", &format!("[\"{v}\"]"));
    }
    if kind == "mod" {
        if let Some(l) = &ctx.loader {
            req = req.query("loaders", &format!("[\"{l}\"]"));
        }
    }
    let list: Value = req.call()?.into_json()?;
    let version = list.as_array().and_then(|a| a.first()).ok_or_else(|| anyhow!("no compatible version found"))?;

    let file = version["files"]
        .as_array()
        .and_then(|f| f.iter().find(|f| f["primary"] == true).or_else(|| f.first()))
        .ok_or_else(|| anyhow!("no files in version"))?;
    let name = file["filename"].as_str().unwrap_or_default();
    let url = file["url"].as_str().unwrap_or_default();

    let bad_name = name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..");
    if bad_name || !url.starts_with("https://cdn.modrinth.com/") {
        bail!("bad file entry");
    }

    let dir = if kind == "mod" { paths::mods_dir() } else { paths::packs_dir() };
    versions::download_to(url, &dir.join(name))?;
    done.push(name.to_string());
    record_installed(kind, id, name);

    if let Some(deps) = version["dependencies"].as_array() {
        for d in deps.iter().filter(|d| d["dependency_type"] == "required") {
            if let Some(pid) = d["project_id"].as_str() {
                install(pid, kind, ctx, seen, done, depth + 1)?;
            }
        }
    }
    Ok(())
}