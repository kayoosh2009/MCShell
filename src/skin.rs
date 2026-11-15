use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::paths;

pub fn set_skin(source: &Path) -> Result<PathBuf> {
    if !source.is_file() {
        bail!("file not found: {}", source.display());
    }

    let bytes = fs::read(source)?;
    if bytes.len() < 8 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        bail!("not a valid PNG file");
    }

    fs::create_dir_all(paths::data_dir())?;
    let dest = paths::skin_file();
    fs::copy(source, &dest)?;

    let hist_dir = history_dir();
    let _ = fs::create_dir_all(&hist_dir);
    if let Some(fname) = source.file_name() {
        let _ = fs::copy(source, hist_dir.join(fname));
    }
    Ok(dest)
}

pub fn has_skin() -> bool {
    paths::skin_file().is_file()
}

fn history_dir() -> PathBuf {
    paths::data_dir().join("skins_history")
}

pub fn list_history() -> Vec<(String, PathBuf)> {
    let dir = history_dir();
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut items: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("png")).unwrap_or(false))
        .filter_map(|p| {
            let name = p.file_stem()?.to_str()?.to_string();
            Some((name, p))
        })
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

pub fn select_history(name: &str) -> Result<()> {
    let src = history_dir().join(format!("{name}.png"));
    if !src.is_file() {
        bail!("skin not found: {name}");
    }
    fs::create_dir_all(paths::data_dir())?;
    fs::copy(src, paths::skin_file())?;
    Ok(())
}