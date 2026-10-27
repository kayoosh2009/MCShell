use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::paths;

// Глобальное состояние прогресса для веб-интерфейса
pub static IS_DOWNLOADING: AtomicBool = AtomicBool::new(false);
pub static DOWNLOADED_BYTES: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_BYTES: AtomicU64 = AtomicU64::new(0);
pub static SPEED_BPS: AtomicU64 = AtomicU64::new(0);
pub static CURRENT_VERSION_TASK: Mutex<String> = Mutex::new(String::new());

pub fn get_progress_json() -> String {
    let active = IS_DOWNLOADING.load(Ordering::Relaxed);
    let downloaded = DOWNLOADED_BYTES.load(Ordering::Relaxed);
    let total = TOTAL_BYTES.load(Ordering::Relaxed);
    let speed = SPEED_BPS.load(Ordering::Relaxed);
    let task = CURRENT_VERSION_TASK.lock().unwrap().clone();

    format!(
        r#"{{"active":{active},"downloaded":{downloaded},"total":{total},"speed":{speed},"task":"{task}"}}"#
    )
}

pub struct VersionEntry {
    pub id: String,
    pub kind: String,
    pub url: String,
}

pub fn fetch_manifest() -> Result<Vec<VersionEntry>> {
    let url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    let body: Value = ureq::get(url).call()?.into_json()?;
    let arr = body["versions"].as_array().ok_or_else(|| anyhow!("bad manifest"))?;
    Ok(arr
        .iter()
        .map(|v| VersionEntry {
            id: v["id"].as_str().unwrap_or_default().to_string(),
            kind: v["type"].as_str().unwrap_or_default().to_string(),
            url: v["url"].as_str().unwrap_or_default().to_string(),
        })
        .collect())
}

pub fn installed_versions() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(paths::versions_dir()) {
        for e in entries.flatten() {
            if let Some(name) = e.file_name().to_str() {
                if e.path().join(format!("{name}.json")).is_file() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

pub(crate) fn download_to(url: &str, dest: &PathBuf) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let resp = ureq::get(url).call()?;
    if let Some(len_str) = resp.header("Content-Length") {
        if let Ok(len) = len_str.parse::<u64>() {
            TOTAL_BYTES.fetch_add(len, Ordering::Relaxed);
        }
    }

    let mut reader = resp.into_reader();
    let mut file = fs::File::create(dest)?;
    let mut buf = [0u8; 16384];

    let mut last_instant = Instant::now();
    let mut bytes_since_last = 0u64;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        DOWNLOADED_BYTES.fetch_add(n as u64, Ordering::Relaxed);
        bytes_since_last += n as u64;

        let elapsed = last_instant.elapsed();
        if elapsed.as_millis() >= 250 {
            let speed = (bytes_since_last as f64 / elapsed.as_secs_f64()) as u64;
            SPEED_BPS.store(speed, Ordering::Relaxed);
            last_instant = Instant::now();
            bytes_since_last = 0;
        }
    }

    Ok(())
}

pub fn library_allowed(lib: &Value) -> bool {
    match lib.get("rules").and_then(|r| r.as_array()) {
        None => true,
        Some(rules) => {
            let mut allowed = false;
            for rule in rules {
                let action_allow = rule.get("action").and_then(|a| a.as_str()) == Some("allow");
                let os_ok = match rule.get("os").and_then(|o| o.get("name")).and_then(|n| n.as_str()) {
                    Some(name) => name == "linux",
                    None => true,
                };
                if os_ok {
                    allowed = action_allow;
                }
            }
            allowed
        }
    }
}

pub fn download_libraries(libraries: &Value, progress: &dyn Fn(String)) -> Result<()> {
    let libs_dir = paths::libraries_dir();
    let Some(arr) = libraries.as_array() else { return Ok(()) };
    for lib in arr {
        if !library_allowed(lib) {
            continue;
        }

        // 1. Стандартные библиотеки Mojang с блоком downloads
        if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
            if let (Some(path), Some(url)) = (
                artifact.get("path").and_then(|p| p.as_str()),
                artifact.get("url").and_then(|u| u.as_str()),
            ) {
                progress(format!("library: {path}"));
                download_to(url, &libs_dir.join(path))?;
                
                // Нативы Mojang
                if let Some(native) = lib
                    .get("downloads")
                    .and_then(|d| d.get("classifiers"))
                    .and_then(|c| c.get("natives-linux"))
                {
                    if let (Some(n_path), Some(n_url)) = (
                        native.get("path").and_then(|p| p.as_str()),
                        native.get("url").and_then(|u| u.as_str()),
                    ) {
                        progress(format!("native: {n_path}"));
                        download_to(n_url, &libs_dir.join(n_path))?;
                    }
                }
                continue;
            }
        }

        // 2. Библиотеки Fabric без блока downloads (скачиваем по Maven name)
        if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
            let base_url = lib.get("url").and_then(|u| u.as_str()).unwrap_or("https://maven.fabricmc.net/");
            // Гарантируем, что URL заканчивается на /
            let base_url = if base_url.ends_with('/') { 
                base_url.to_string() 
            } else { 
                format!("{base_url}/") 
            };
            
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() >= 3 {
                let group = parts[0].replace('.', "/");
                let artifact = parts[1];
                let version = parts[2];
                let filename = if parts.len() >= 4 {
                    format!("{artifact}-{version}-{}.jar", parts[3])
                } else {
                    format!("{artifact}-{version}.jar")
                };
                let rel_path = format!("{group}/{artifact}/{version}/{filename}");
                let full_url = format!("{base_url}{rel_path}");
                progress(format!("fabric library: {artifact}-{version}"));
                download_to(&full_url, &libs_dir.join(rel_path))?;
            }
        }
    }
    Ok(())
}

fn download_assets(version_json: &Value, progress: &dyn Fn(String)) -> Result<()> {
    let Some(asset_index) = version_json.get("assetIndex") else { return Ok(()) };
    let id = asset_index["id"].as_str().unwrap_or("legacy");
    let url = asset_index["url"].as_str().unwrap_or_default();

    let indexes_dir = paths::assets_dir().join("indexes");
    let index_path = indexes_dir.join(format!("{id}.json"));
    download_to(url, &index_path)?;

    let index: Value = serde_json::from_slice(&fs::read(&index_path)?)?;
    let objects = index["objects"].as_object().ok_or_else(|| anyhow!("bad asset index"))?;
    let total = objects.len();
    let objects_dir = paths::assets_dir().join("objects");

    for (i, (_name, obj)) in objects.iter().enumerate() {
        let hash = obj["hash"].as_str().unwrap_or_default();
        if hash.len() < 2 {
            continue;
        }
        let dest = objects_dir.join(&hash[0..2]).join(hash);
        if dest.is_file() {
            continue;
        }
        let url = format!("https://resources.download.minecraft.net/{}/{}", &hash[0..2], hash);
        download_to(&url, &dest)?;
        if i % 25 == 0 {
            progress(format!("assets: {i}/{total}"));
        }
    }
    progress("assets: done".to_string());
    Ok(())
}

pub fn install_version(id: &str, progress: &dyn Fn(String)) -> Result<()> {
    IS_DOWNLOADING.store(true, Ordering::Relaxed);
    DOWNLOADED_BYTES.store(0, Ordering::Relaxed);
    TOTAL_BYTES.store(0, Ordering::Relaxed);
    SPEED_BPS.store(0, Ordering::Relaxed);
    if let Ok(mut task) = CURRENT_VERSION_TASK.lock() {
        *task = id.to_string();
    }

    let result = (|| -> Result<()> {
        let manifest = fetch_manifest()?;
        let entry = manifest.iter().find(|v| v.id == id).ok_or_else(|| anyhow!("version {id} not found"))?;

        let version_dir = paths::versions_dir().join(id);
        let json_path = version_dir.join(format!("{id}.json"));
        progress(format!("fetching {id}.json"));
        download_to(&entry.url, &json_path)?;

        let version_json: Value = serde_json::from_slice(&fs::read(&json_path)?)?;

        if let Some(url) = version_json["downloads"]["client"]["url"].as_str() {
            progress("downloading client jar".to_string());
            download_to(url, &version_dir.join(format!("{id}.jar")))?;
        }

        if let Some(libs) = version_json.get("libraries") {
            progress("downloading libraries".to_string());
            download_libraries(libs, progress)?;
        }

        progress("downloading assets".to_string());
        download_assets(&version_json, progress)?;

        progress(format!("{id}: install complete"));
        Ok(())
    })();

    IS_DOWNLOADING.store(false, Ordering::Relaxed);
    SPEED_BPS.store(0, Ordering::Relaxed);
    result
}

pub fn delete_version(id: &str) -> Result<()> {
    let version_dir = paths::versions_dir().join(id);
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)?;
    }
    Ok(())
}