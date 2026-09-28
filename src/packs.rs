use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::paths;

pub fn list() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(paths::packs_dir()) {
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let is_zip = p.is_file() && name.ends_with(".zip");
            let is_dir = p.is_dir() && p.join("pack.mcmeta").is_file();
            if is_zip || is_dir {
                out.push(name);
            }
        }
    }
    out.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    out
}

pub fn add(source: &Path) -> Result<String> {
    if !source.is_file() {
        bail!("file not found: {}", source.display());
    }
    if source.extension().and_then(|e| e.to_str()) != Some("zip") {
        bail!("not a .zip file");
    }
    let name = source.file_name().unwrap().to_string_lossy().to_string();
    fs::create_dir_all(paths::packs_dir())?;
    fs::copy(source, paths::packs_dir().join(&name))?;
    Ok(name)
}

pub fn remove(name: &str) -> Result<()> {
    let p = paths::packs_dir().join(name);
    if p.is_dir() {
        fs::remove_dir_all(p)?;
    } else {
        fs::remove_file(p)?;
    }
    Ok(())
}