use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::paths;

#[derive(Clone)]
pub struct ModEntry {
    pub name: String,
    pub enabled: bool,
}

pub fn list() -> Vec<ModEntry> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(paths::mods_dir()) {
        for e in entries.flatten() {
            let file = e.file_name().to_string_lossy().to_string();
            if file.ends_with(".jar") {
                out.push(ModEntry { name: file, enabled: true });
            } else if let Some(base) = file.strip_suffix(".disabled") {
                if base.ends_with(".jar") {
                    out.push(ModEntry { name: base.to_string(), enabled: false });
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn add(source: &Path) -> Result<String> {
    if !source.is_file() {
        bail!("file not found: {}", source.display());
    }
    if source.extension().and_then(|e| e.to_str()) != Some("jar") {
        bail!("not a .jar file");
    }
    let name = source.file_name().unwrap().to_string_lossy().to_string();
    fs::create_dir_all(paths::mods_dir())?;
    fs::copy(source, paths::mods_dir().join(&name))?;
    Ok(name)
}

pub fn toggle(m: &ModEntry) -> Result<()> {
    let dir = paths::mods_dir();
    let enabled = dir.join(&m.name);
    let disabled = dir.join(format!("{}.disabled", m.name));
    if m.enabled {
        fs::rename(enabled, disabled)?;
    } else {
        fs::rename(disabled, enabled)?;
    }
    Ok(())
}

pub fn remove(m: &ModEntry) -> Result<()> {
    let dir = paths::mods_dir();
    let path = if m.enabled {
        dir.join(&m.name)
    } else {
        dir.join(format!("{}.disabled", m.name))
    };
    fs::remove_file(path)?;
    Ok(())
}