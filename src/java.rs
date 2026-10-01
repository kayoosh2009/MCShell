use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};

use crate::{paths, versions};

/// (major, подсказка). Реальную нужную версию лаунчер берёт из JSON игры.
pub const JAVAS: [(u32, &str); 4] = [
    (8, "up to 1.16.5"),
    (17, "1.17 - 1.20.4"),
    (21, "1.20.5 - 1.21.x"),
    (25, "newest versions (26.x)"),
];

pub fn dir(major: u32) -> PathBuf {
    paths::data_dir().join("java").join(major.to_string())
}

pub fn bin(major: u32) -> PathBuf {
    dir(major).join("bin/java")
}

pub fn is_installed(major: u32) -> bool {
    bin(major).is_file()
}

/// Подбирает java: точная версия, иначе ближайшая старше, иначе системная.
pub fn path_for(need: Option<u64>) -> String {
    let need = need.unwrap_or(0) as u32;
    let best = JAVAS.iter().map(|j| j.0).filter(|&m| m >= need && is_installed(m)).min();
    match best {
        Some(m) => bin(m).to_string_lossy().to_string(),
        None => "java".to_string(),
    }
}

pub fn install(major: u32, progress: &dyn Fn(String)) -> Result<()> {
    if is_installed(major) {
        progress(format!("java {major} already installed"));
        return Ok(());
    }
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        a => bail!("unsupported arch: {a}"),
    };
    let url = format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/linux/{arch}/jre/hotspot/normal/eclipse");
    let tmp = paths::data_dir().join("java").join(format!("{major}.tar.gz"));

    progress(format!("downloading java {major}..."));
    versions::download_to(&url, &tmp)?;

    let dest = dir(major);
    let _ = fs::remove_dir_all(&dest);
    fs::create_dir_all(&dest)?;
    progress("extracting...".to_string());
    let out = Command::new("tar")
        .arg("-xzf").arg(&tmp)
        .arg("-C").arg(&dest)
        .arg("--strip-components=1")
        .output()?;
    let _ = fs::remove_file(&tmp);
    if !out.status.success() {
        let _ = fs::remove_dir_all(&dest);
        bail!("tar failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    if !is_installed(major) {
        bail!("java binary not found after extract");
    }
    progress(format!("java {major}: install complete"));
    Ok(())
}

pub fn remove(major: u32) -> Result<()> {
    let d = dir(major);
    if d.exists() {
        fs::remove_dir_all(d)?;
    }
    Ok(())
}