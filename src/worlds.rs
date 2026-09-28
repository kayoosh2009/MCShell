use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use zip::write::SimpleFileOptions;

use crate::paths;

pub fn list() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(paths::saves_dir()) {
        for e in entries.flatten() {
            let p = e.path();
            if p.join("level.dat").is_file() {
                if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                    out.push(n.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

fn add_dir(
    zip: &mut zip::ZipWriter<File>,
    base: &Path,
    dir: &Path,
    prefix: &str,
    opts: SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let rel = path.strip_prefix(base)?;
        let name = format!("{prefix}/{}", rel.to_string_lossy());
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), opts)?;
            add_dir(zip, base, &path, prefix, opts)?;
        } else {
            zip.start_file(name, opts)?;
            let mut f = File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}

pub fn export(name: &str) -> Result<PathBuf> {
    let src = paths::saves_dir().join(name);
    if !src.is_dir() {
        bail!("world not found: {name}");
    }
    let dest = paths::home_dir().join(format!("{name}.zip"));
    let mut zip = zip::ZipWriter::new(File::create(&dest)?);
    let opts = SimpleFileOptions::default();
    zip.add_directory(format!("{name}/"), opts)?;
    add_dir(&mut zip, &src, &src, name, opts)?;
    zip.finish()?;
    Ok(dest)
}

fn find_world_root(tmp: &Path, zip_path: &Path) -> Result<(PathBuf, String)> {
    if tmp.join("level.dat").is_file() {
        let name = zip_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "world".to_string());
        return Ok((tmp.to_path_buf(), name));
    }
    for e in fs::read_dir(tmp)? {
        let p = e?.path();
        if p.join("level.dat").is_file() {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            return Ok((p, name));
        }
    }
    bail!("no level.dat found: not a Minecraft world")
}

pub fn import(zip_path: &Path) -> Result<String> {
    if !zip_path.is_file() {
        bail!("file not found: {}", zip_path.display());
    }
    let tmp = paths::data_dir().join("import_tmp");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp)?;

    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    archive.extract(&tmp)?;

    let result = find_world_root(&tmp, zip_path).and_then(|(root, name)| {
        fs::create_dir_all(paths::saves_dir())?;
        let dest = paths::saves_dir().join(&name);
        if dest.exists() {
            bail!("world '{name}' already exists");
        }
        fs::rename(&root, &dest)?;
        Ok(name)
    });

    let _ = fs::remove_dir_all(&tmp);
    result
}