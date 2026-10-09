use std::fs::{self, File};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

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
    dir(major).join("bin").join("java.exe")
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
    let url = format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/{arch}/jre/hotspot/normal/eclipse");
    let base = paths::data_dir().join("java");
    let tmp = base.join(format!("{major}.zip"));
    let tmp_dir = base.join(format!("{major}_tmp"));

    progress(format!("downloading java {major}..."));
    versions::download_to(&url, &tmp)?;

    progress("extracting...".to_string());
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir)?;
    let result = (|| -> Result<()> {
        zip::ZipArchive::new(File::open(&tmp)?)?.extract(&tmp_dir)?;
        let root = fs::read_dir(&tmp_dir)?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .ok_or_else(|| anyhow!("empty archive"))?;
        let dest = dir(major);
        let _ = fs::remove_dir_all(&dest);
        fs::rename(root, dest)?;
        Ok(())
    })();
    let _ = fs::remove_file(&tmp);
    let _ = fs::remove_dir_all(&tmp_dir);
    result?;

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