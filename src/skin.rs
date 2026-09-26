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
    Ok(dest)
}

pub fn has_skin() -> bool {
    paths::skin_file().is_file()
}